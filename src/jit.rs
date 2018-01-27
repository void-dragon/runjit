use pest::Parser;
use pest::iterators::Pair;

use llvm;
use llvm::core::*;
use llvm::prelude::*;
use llvm::execution_engine::*;
use llvm::target::*;

use std::collections::BTreeMap;
use std::fs::File;
use std::ffi::{CStr, CString};
use std::io::Read;
use std::mem;
use std::rc::Rc;
use std::ptr;

use libc;

use parser::*;

#[derive(Debug)]
enum Value {
    Array(Vec<Rc<Value>>),
    Dict,
    Lambda,
    Float(f64),
    Str(CString),
    Null,
}

pub struct Context {
    llvm_ctx: LLVMContextRef,
    llvm_f64: LLVMTypeRef,
    llvm_ptr: LLVMTypeRef,
    llvm_ctx_ptr: LLVMValueRef,
    self_ptr: *const Context,
    llvm_builder: LLVMBuilderRef,
    llvm_module: LLVMModuleRef,
    extern_functions: BTreeMap<String, (LLVMValueRef, *mut libc::c_void)>,
    runtime_variables: BTreeMap<CString, Rc<Value>>,
}

extern "C" fn global_get(name: *const Value) -> *const Value {
    println!("!! get !!");
    &Value::Null as *const _
}

unsafe extern "C" fn global_set(ctx: *mut Context, name: *const Value, val: *mut Value) -> *const Value {
    println!("!! set {:?} = {:?} !!", *name, *val);

    if let Value::Array(ref a) = *name {
        if let Value::Str(ref s) = *a[0] {
            (*ctx).runtime_variables.insert(s.clone(), Rc::from_raw(val));
        }
    }

    &Value::Null as *const _
}

extern "C" fn array_new() -> *const Value {
    println!("!! new array !!");
    Rc::into_raw(Rc::new(Value::Array(Vec::new())))
}

unsafe extern "C" fn array_push(arr: *mut Value, v: *mut Value) -> *const Value {
    println!("!! pushing value !! {:?}", *arr);

    if let Value::Array(ref mut a) = *arr {
        a.push(Rc::from_raw(v));

        println!("{:?}", a);
    }

    &Value::Null as *const _
}

extern "C" fn string_new() -> *const Value {
    println!("!! new string !!");
    Rc::into_raw(Rc::new(Value::Str(CString::new("").unwrap())))
}

unsafe extern "C" fn string_from(bytes: *mut libc::c_char) -> *const Value {
    println!("!! string from !!");
    // let data = CString::from_raw(bytes);
    let data = CStr::from_ptr(bytes);
    println!("created");
    Rc::into_raw(Rc::new(Value::Str(data.to_owned())))
}

extern "C" fn float_new(v: f64) -> *const Value {
    println!("!! new float {} !!", v);
    Rc::into_raw(Rc::new(Value::Float(v)))
}

impl Context {
    pub fn new() -> Box<Context> {
        unsafe {
            let context = LLVMContextCreate();

            let mut ctx = Box::new(Context {
                llvm_ctx: context,
                llvm_f64: LLVMDoubleTypeInContext(context),
                llvm_ptr: LLVMInt64TypeInContext(context),
                llvm_ctx_ptr: 0 as *mut _,
                self_ptr: 0 as *const Context,
                llvm_builder: LLVMCreateBuilderInContext(context),
                llvm_module: LLVMModuleCreateWithNameInContext(
                    b"__module__\0".as_ptr() as *const _,
                    context,
                ),
                extern_functions: BTreeMap::new(),
                runtime_variables: BTreeMap::new(),
            });

            ctx.llvm_ctx_ptr = LLVMAddGlobal(
                ctx.llvm_module,
                ctx.llvm_ptr,
                b"__context\0".as_ptr() as *const _,
            );
            ctx.self_ptr = &*ctx as *const Context;

            ctx.add_fn("__global_get", global_get as *mut _, 2);
            ctx.add_fn("__global_set", global_set as *mut _, 3);
            ctx.add_fn("__array_new", array_new as *mut _, 0);
            ctx.add_fn("__array_push", array_push as *mut _, 2);
            ctx.add_fn("__string_new", string_new as *mut _, 0);
            ctx.add_fn("__string_from", string_from as *mut _, 1);

            let mut args = Vec::new();
            args.push(ctx.llvm_f64);

            let ft = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
            let func = LLVMAddFunction(ctx.llvm_module, CString::new("__float_new").unwrap().as_ptr(), ft);

            ctx.extern_functions.insert("__float_new".to_string(), (func, float_new as *mut _));

            ctx
        }
    }

    pub fn read_file(&self, filename: &str) {
        let mut file = File::open(filename).unwrap();
        let mut source = String::new();

        file.read_to_string(&mut source).unwrap();

        let pair = RunjitParser::parse(Rule::input, &source)
            .unwrap_or_else(|e| panic!("{}", e))
            .next()
            .unwrap();

        unsafe {
            let main_func_t = LLVMFunctionType(LLVMVoidType(), ptr::null_mut(), 0, 0);
            let main = LLVMAddFunction(
                self.llvm_module,
                b"__main__\0".as_ptr() as *const _,
                main_func_t,
            );

            let bb = LLVMAppendBasicBlockInContext(
                self.llvm_ctx,
                main,
                b"__main__entry\0".as_ptr() as *const _,
            );

            LLVMPositionBuilderAtEnd(self.llvm_builder, bb);

            consume(&self, pair);

            LLVMBuildRetVoid(self.llvm_builder);

            LLVMDisposeBuilder(self.llvm_builder);

            LLVMDumpModule(self.llvm_module);
        }
    }

    pub fn add_fn(&mut self, name: &str, f: *mut libc::c_void, cnt: u32) {
        let mut args = Vec::new();
        for _ in 0..cnt {
            args.push(self.llvm_ptr);
        }

        let func = unsafe {
            let ft = LLVMFunctionType(self.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
            LLVMAddFunction(self.llvm_module, CString::new(name).unwrap().as_ptr(), ft)
        };

        self.extern_functions.insert(name.to_string(), (func, f));
    }

    pub fn run(&mut self) {
        unsafe {
            let mut ee = mem::uninitialized();
            let mut out = mem::zeroed();

            LLVMLinkInMCJIT();
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();

            LLVMCreateExecutionEngineForModule(&mut ee, self.llvm_module, &mut out);

            LLVMAddGlobalMapping(ee, self.llvm_ctx_ptr, self.self_ptr as *mut _);

            for (name, &mut (valref, func)) in self.extern_functions.iter_mut() {
                LLVMAddGlobalMapping(ee, valref, func);
            }

            let addr = LLVMGetFunctionAddress(ee, b"__main__\0".as_ptr() as *const _);

            let f: extern "C" fn() = mem::transmute(addr);

            f();

            LLVMDisposeExecutionEngine(ee);
        }
    }

    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.runtime_variables.get(&CString::new(name).unwrap()).and_then(|x| {
            if let Value::Float(f) = **x {
                Some(f)
            } else {
                None
            }
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.llvm_module);
            LLVMContextDispose(self.llvm_ctx);
        }
    }
}

fn consume(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    match pair.as_rule() {
        Rule::block => block(ctx, pair),
        Rule::statement => statement(ctx, pair),
        _ => panic!("unexpected token"),
    }
}

fn block(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let mut v = Vec::new();

    for pair in pair.into_inner() {
        v.push(consume(ctx, pair));
    }

    0 as LLVMValueRef
}

fn statement(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let next = pair.into_inner().next().unwrap();

    match next.as_rule() {
        Rule::assign => assign(ctx, next),
        Rule::call => call(ctx, next),
        //     // Rule::_if => _if(next),
        _ => panic!("unrecognized statement: {:?}", next.as_rule()),
    }
}

fn access(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let inner = pair.into_inner();

    let an_ref = unsafe {
        let an = ctx.extern_functions.get("__array_new").unwrap();

        LLVMBuildCall(
            ctx.llvm_builder,
            an.0,
            0 as *mut LLVMValueRef,
            0,
            b"__array_new\0".as_ptr() as *const _,
        )
    };

    let string_from = ctx.extern_functions.get("__string_from").unwrap();
    let array_push = ctx.extern_functions.get("__array_push").unwrap();

    for p in inner {
        print!("{:?} ", p.as_str());
        let x = match p.as_rule() {
            Rule::ident => unsafe {
                let s = p.as_str();
                let cstr = LLVMBuildGlobalStringPtr(
                    ctx.llvm_builder,
                    CString::new(s).unwrap().as_ptr(),
                    b"__str\0".as_ptr() as *const _,
                );

                let args = vec![cstr];
                LLVMBuildCall(
                    ctx.llvm_builder,
                    string_from.0,
                    args.as_ptr() as *mut LLVMValueRef,
                    args.len() as u32,
                    b"__string_from\0".as_ptr() as *const _,
                )
            },
            Rule::exp => unsafe { exp(ctx, p) },
            _ => panic!("unexpected in access rule"),
        };

        let args = vec![an_ref, x];
        unsafe {
            LLVMBuildCall(
                ctx.llvm_builder,
                array_push.0,
                args.as_ptr() as *mut LLVMValueRef,
                args.len() as u32,
                b"__array_push\0".as_ptr() as *const _,
            )
        };
    }
    print!("\n");

    an_ref
}

unsafe fn exp(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let mut inner = pair.into_inner();

    let next = inner.next().unwrap();
    let rule = next.as_rule();
    let left_ref = match rule {
        Rule::exp => exp(ctx, next),
        Rule::literal => {
            let inner = next.into_inner().next().unwrap();

            match inner.as_rule() {
                Rule::numeric => {
                    let s = inner.as_str().trim();
                    println!("[{}]", s);
                    println!("{:?}", inner);
                    // LLVMConstRealOfString(ctx.llvm_f64, CString::new(s).unwrap().as_ptr())

                    let float_new = ctx.extern_functions.get("__float_new").unwrap();
                    let args = vec![LLVMConstReal(ctx.llvm_f64, s.parse().unwrap())];
                    LLVMBuildCall(
                        ctx.llvm_builder,
                        float_new.0,
                        args.as_ptr() as *mut LLVMValueRef,
                        args.len() as u32,
                        b"__float_new\0".as_ptr() as *const _,
                    )
                }
                _ => panic!("not supported yet"),
            }
        }
        Rule::access => access(ctx, next),
        _ => panic!("unknown exp: {:?}", rule),
    };

    if let Some(op) = inner.next() {
        if let Some(right) = inner.next() {
            let right_ref = exp(ctx, right);

            match op.as_rule() {
                Rule::op_add => {
                    LLVMBuildFAdd(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_add\0".as_ptr() as *const _,
                    )
                }
                Rule::op_sub => {
                    LLVMBuildFSub(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_sub\0".as_ptr() as *const _,
                    )
                }
                Rule::op_mul => {
                    LLVMBuildFMul(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_mul\0".as_ptr() as *const _,
                    )
                }
                Rule::op_div => {
                    LLVMBuildFDiv(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_div\0".as_ptr() as *const _,
                    )
                }
                // Rule::op_mod => Operation::Mod,
                Rule::op_and => {
                    LLVMBuildAnd(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_and\0".as_ptr() as *const _,
                    )
                }
                Rule::op_or => {
                    LLVMBuildOr(
                        ctx.llvm_builder,
                        left_ref,
                        right_ref,
                        b"exp_or\0".as_ptr() as *const _,
                    )
                }
                Rule::op_eq => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealOEQ,
                        left_ref,
                        right_ref,
                        b"exp_oeq\0".as_ptr() as *const _,
                    )
                }
                Rule::op_neq => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealONE,
                        left_ref,
                        right_ref,
                        b"exp_one\0".as_ptr() as *const _,
                    )
                }
                Rule::op_gt => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealOGT,
                        left_ref,
                        right_ref,
                        b"exp_ogt\0".as_ptr() as *const _,
                    )
                }
                Rule::op_le => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealOLT,
                        left_ref,
                        right_ref,
                        b"exp_olt\0".as_ptr() as *const _,
                    )
                }
                Rule::op_gte => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealOGE,
                        left_ref,
                        right_ref,
                        b"exp_oge\0".as_ptr() as *const _,
                    )
                }
                Rule::op_lee => {
                    LLVMBuildFCmp(
                        ctx.llvm_builder,
                        llvm::LLVMRealPredicate::LLVMRealOLE,
                        left_ref,
                        right_ref,
                        b"exp_ole\0".as_ptr() as *const _,
                    )
                }
                _ => panic!("unknown operation in expression: {:?}", op.as_rule()),
            }
        } else {
            panic!("incomplete expression")
        }
    } else {
        left_ref
    }

}

fn assign(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let mut inner = pair.into_inner();

    let v = inner.next().unwrap();
    let ident_array = match v.as_rule() {
        Rule::access => access(ctx, v),
        _ => panic!("expected access"),
    };

    let e = inner.next().unwrap();
    let ex = match e.as_rule() {
        Rule::exp => unsafe { exp(ctx, e) }
        // Rule::lambda => lambda(e),
        // Rule::dict => dict(e),
        // Rule::array => array(e),
        _ => panic!("unexpected assign: {:?}", e.as_rule()),
    };

    let global_set = ctx.extern_functions.get("__global_set").unwrap();
    let args = vec![ctx.llvm_ctx_ptr, ident_array, ex];
    unsafe {
        LLVMBuildCall(
            ctx.llvm_builder,
            global_set.0,
            args.as_ptr() as *mut LLVMValueRef,
            args.len() as u32,
            b"__global_set\0".as_ptr() as *const _,
        )
    }
}
//
// fn lambda(pair: Pair<Rule>) {
//     let inner = pair.into_inner();
//     let mut names = Vec::new();
//     let mut statements = Vec::new();
//
//     for node in inner {
//         match node.as_rule() {
//             Rule::names => {
//                 let inner = node.into_inner();
//
//                 for node in inner {
//                     names.push(String::from(node.as_str()));
//                 }
//             }
//             Rule::block => {
//                 if let Ok(Ast::Block(stmnts)) = Rc::try_unwrap(block(node)) {
//                     statements = stmnts;
//                 }
//             }
//             _ => panic!("unexpected element"),
//         }
//     }
//
//     Rc::new(Ast::Lambda(names, statements))
// }

fn call(ctx: &Context, pair: Pair<Rule>) -> LLVMValueRef {
    let mut call = pair.into_inner();

    // let name = String::from(call.next().unwrap().as_str());
    //
    // let mut params = Vec::new();
    //
    // if let Some(ps) = call.next() {
    //     for mut param in ps.into_inner() {
    //         match param.as_rule() {
    //             Rule::exp => params.push(exp(param)),
    //             _ => panic!("unexpected stuff"),
    //         }
    //     }
    // }

    // unsafe {
    //     LLVMBuildCall(
    //         ctx.llvm_builder,
    //         val.0,
    //         0 as *mut LLVMValueRef,
    //         0,
    //         b"myprint\0".as_ptr() as *const _,
    //     )
    // }
    0 as LLVMValueRef
}

// fn _if(pair: Pair<Rule>) {
//     let mut the_if = pair.into_inner();
//
//     let cond = exp(the_if.next().unwrap());
//     let block = if let Ok(Ast::Block(b)) = Rc::try_unwrap(block(the_if.next().unwrap())) {
//         b
//     } else {
//         Vec::new()
//     };
//     let elsy = match the_if.next() {
//         Some(block) => _else(block),
//         None => Rc::new(Ast::Nothing),
//     };
//
//     Rc::new(Ast::If(cond, block, elsy))
// }
//
// fn _else(pair: Pair<Rule>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
//
// fn dict(pair: Pair<Rule>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
//
// fn array(pair: Pair<Rule>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
