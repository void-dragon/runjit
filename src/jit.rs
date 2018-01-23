use pest::Parser;
use pest::inputs::{Input, FileInput};
use pest::iterators::Pair;

use llvm::core::*;
use llvm::prelude::*;
use llvm::execution_engine::*;
use llvm::target::*;

use std::collections::BTreeMap;
use std::ffi::CString;
use std::rc::Rc;
use std::mem;

use libc;

use parser::*;

pub struct Context {
    llvm_ctx: LLVMContextRef,
    llvm_f64: LLVMTypeRef,
    llvm_builder: LLVMBuilderRef,
    llvm_module: LLVMModuleRef,
    extern_functions: BTreeMap<String, (LLVMValueRef, *mut libc::c_void)>,
}

impl Context {
    pub fn new() -> Context {
        unsafe {
            let context = LLVMContextCreate();

            Context {
                llvm_ctx: context,
                llvm_f64: LLVMDoubleTypeInContext(context),
                llvm_builder: LLVMCreateBuilderInContext(context),
                llvm_module: LLVMModuleCreateWithNameInContext(
                    b"__module__\0".as_ptr() as *const _,
                    context,
                ),
                extern_functions: BTreeMap::new(),
            }
        }
    }

    pub fn read_file(&self, filename: &str) {
        let source = FileInput::new(filename).unwrap();

        let pair = RunjitParser::parse(Rule::input, Rc::new(source))
            .unwrap_or_else(|e| panic!("{}", e))
            .next()
            .unwrap();

        unsafe {
            let main_func_t = LLVMFunctionType(LLVMVoidType(), 0 as *mut LLVMTypeRef, 0, 0);
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

    pub fn add_fn(&mut self, name: &str, f: *mut libc::c_void) {
        let func = unsafe {
            let ft = LLVMFunctionType(LLVMVoidType(), 0 as *mut LLVMTypeRef, 0, 0);
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

            println!("create engine");
            LLVMCreateExecutionEngineForModule(&mut ee, self.llvm_module, &mut out);

            println!("set global mapping");
            for (name, &mut (valref, func)) in self.extern_functions.iter_mut() {
                // LLVMAddGlobalMapping(ee, valref, mem::transmute(func));
                // LLVMAddGlobalMapping(ee, valref, func as *mut _);
                LLVMAddGlobalMapping(ee, valref, func);
            }

            let addr = LLVMGetFunctionAddress(ee, b"__main__\0".as_ptr() as *const _);

            println!("run __main__({})", addr);
            let f: extern "C" fn() = mem::transmute(addr);

            f();
            // let mut func: LLVMValueRef = 0 as LLVMValueRef;
            // LLVMFindFunction(ee, b"__main__\0".as_ptr() as *const _, &mut func);
            // LLVMRunFunction(ee, func, 0, 0 as *mut _);

            println!("dispose engine");
            LLVMDisposeExecutionEngine(ee);
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            LLVMContextDispose(self.llvm_ctx);
        }
    }
}

fn consume<I: Input>(ctx: &Context, pair: Pair<Rule, I>) -> LLVMValueRef {
    match pair.as_rule() {
        Rule::block => block(ctx, pair),
        Rule::statement => statement(ctx, pair),
        _ => panic!("unexpected token"),
    }
}

fn block<I: Input>(ctx: &Context, pair: Pair<Rule, I>) -> LLVMValueRef {
    let mut v = Vec::new();

    for pair in pair.into_inner() {
        v.push(consume(ctx, pair));
    }

    0 as LLVMValueRef
}

fn statement<I: Input>(ctx: &Context, pair: Pair<Rule, I>) -> LLVMValueRef {
    let next = pair.into_inner().next().unwrap();

    match next.as_rule() {
        Rule::assign => assign(ctx, next),
        //     // Rule::call => call(next),
        //     // Rule::_if => _if(next),
        _ => panic!("unrecognized statement: {:?}", next.as_rule()),
    }
}

fn access<I: Input>(ctx: &Context, pair: Pair<Rule, I>) -> Vec<String> {
    let inner = pair.into_inner();
    let mut var = Vec::new();

    for p in inner {
        print!("{:?} ", p.as_str());
        let x = match p.as_rule() {
            Rule::ident => String::from(p.as_str()),
            // Rule::exp => exp(ctx, p),
            _ => panic!("unexpected in access rule"),
        };
        var.push(x);
    }
    print!("\n");

    var
}

// fn exp<I: Input>(pair: Pair<Rule, I>) {
//     let mut inner = pair.into_inner();
//
//     let next = inner.next().unwrap();
//     let rule = next.as_rule();
//     let left = match rule {
//         Rule::exp => exp(next),
//         Rule::literal => {
//             let inner = next.into_inner().next().unwrap();
//
//             match inner.as_rule() {
//                 Rule::numeric => {
//                     let s = inner.as_str().trim();
//
//                     let d: f64 = if let Ok(x) = s.parse() {
//                         x
//                     } else {
//                         s.parse::<i64>().unwrap() as f64
//                     };
//
//                     Rc::new(Ast::Float(d))
//                 }
//                 _ => Rc::new(Ast::Str(String::from(inner.as_str()))),
//             }
//         }
//         Rule::access => access(next),
//         _ => panic!("unknown exp: {:?}", rule),
//     };
//
//     if let Some(op) = inner.next() {
//         if let Some(right) = inner.next() {
//
//             let the_op = match op.as_rule() {
//                 Rule::op_add => Operation::Add,
//                 Rule::op_sub => Operation::Sub,
//                 Rule::op_mul => Operation::Mul,
//                 Rule::op_div => Operation::Div,
//                 Rule::op_mod => Operation::Mod,
//                 Rule::op_and => Operation::And,
//                 Rule::op_or => Operation::Or,
//                 Rule::op_eq => Operation::Eq,
//                 Rule::op_neq => Operation::Neq,
//                 Rule::op_gt => Operation::Gt,
//                 Rule::op_le => Operation::Le,
//                 Rule::op_gte => Operation::Gte,
//                 Rule::op_lee => Operation::Lee,
//                 _ => panic!("unknown operation in expression: {:?}", op.as_rule()),
//             };
//
//             Rc::new(Ast::Exp(the_op, left, exp(right)))
//         } else {
//             panic!("incomplete expression")
//         }
//     } else {
//         left
//     }
//
// }
//


fn assign<I: Input>(ctx: &Context, pair: Pair<Rule, I>) -> LLVMValueRef {
    let mut inner = pair.into_inner();

    let v = inner.next().unwrap();
    let ident = match v.as_rule() {
        Rule::access => access(ctx, v),
        _ => panic!("expected access"),
    };

    // let e = inner.next().unwrap();
    // let ex = match e.as_rule() {
    //     Rule::exp => exp(ctx, e),
    //     // Rule::lambda => lambda(e),
    //     // Rule::dict => dict(e),
    //     // Rule::array => array(e),
    //     _ => panic!("unexpected assign: {:?}", e.as_rule()),
    // };

    unsafe {
        // let ft = LLVMFunctionType(LLVMVoidType(), 0 as *mut LLVMTypeRef, 0, 0);
        // let func = LLVMAddFunction(ctx.llvm_module, b"myprint\0".as_ptr() as *const _, ft);
        // let func = LLVMAddGlobal(ctx.llvm_module, ft, b"myprint\0".as_ptr() as *const _);
        let val = ctx.extern_functions.get("myprint").unwrap();

        LLVMBuildCall(
            ctx.llvm_builder,
            val.0,
            0 as *mut LLVMValueRef,
            0,
            b"myprint\0".as_ptr() as *const _,
        )
    }
}
//
// fn lambda<I: Input>(pair: Pair<Rule, I>) {
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
//
// fn call<I: Input>(pair: Pair<Rule, I>) {
//     let mut call = pair.into_inner();
//
//     let name = String::from(call.next().unwrap().as_str());
//
//     let mut params = Vec::new();
//
//     if let Some(ps) = call.next() {
//         for mut param in ps.into_inner() {
//             match param.as_rule() {
//                 Rule::exp => params.push(exp(param)),
//                 _ => panic!("unexpected stuff"),
//             }
//         }
//     }
//
//     Rc::new(Ast::Call(name, params))
// }
//
// fn _if<I: Input>(pair: Pair<Rule, I>) {
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
// fn _else<I: Input>(pair: Pair<Rule, I>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
//
// fn dict<I: Input>(pair: Pair<Rule, I>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
//
// fn array<I: Input>(pair: Pair<Rule, I>) {
//     let mut the_else = pair.into_inner();
//
//     Rc::new(Ast::Nothing)
// }
