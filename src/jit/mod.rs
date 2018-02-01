use pest::Parser;

use llvm::analysis::*;
use llvm::core::*;
use llvm::prelude::*;
use llvm::execution_engine::*;
use llvm::target::*;

use std::collections::BTreeMap;
use std::fs::File;
use std::ffi::CString;
use std::io::Read;
use std::mem;
use std::rc::Rc;
use std::ptr;

use libc;

use parser::*;

mod callbacks;
mod build;
use jit::callbacks::*;

#[derive(Debug)]
pub enum Value {
    Array(Vec<Rc<Value>>),
    Dict(BTreeMap<CString, Rc<Value>>),
    Lambda(u64),
    Float(f64),
    Str(CString),
    Null,
}

// impl Drop for Value {
//     fn drop(&mut self) {
//         println!("droped value: {:?}", self);
//     }
// }

pub struct Context {
    llvm_ctx: LLVMContextRef,
    llvm_f64: LLVMTypeRef,
    llvm_ptr: LLVMTypeRef,
    llvm_ctx_ptr: LLVMValueRef,
    self_ptr: *const Context,
    llvm_builder: LLVMBuilderRef,
    llvm_module: LLVMModuleRef,
    block_stack: Vec<LLVMBasicBlockRef>,
    param_stack: Vec<BTreeMap<String, LLVMValueRef>>,
    extern_functions: BTreeMap<String, (LLVMValueRef, *mut libc::c_void)>,
    runtime_variables: BTreeMap<CString, Rc<Value>>,
}

impl Context {
    pub fn new() -> Box<Context> {
        unsafe {
            let context = LLVMContextCreate();

            let mut ctx = Box::new(Context {
                llvm_ctx: context,
                llvm_f64: LLVMDoubleTypeInContext(context),
                llvm_ptr: LLVMPointerType(LLVMInt64TypeInContext(context), 0),
                llvm_ctx_ptr: 0 as *mut _,
                self_ptr: 0 as *const Context,
                llvm_builder: LLVMCreateBuilderInContext(context),
                llvm_module: LLVMModuleCreateWithNameInContext(
                    b"__main__\0".as_ptr() as *const _,
                    context,
                ),
                block_stack: Vec::new(),
                param_stack: Vec::new(),
                extern_functions: BTreeMap::new(),
                runtime_variables: BTreeMap::new(),
            });

            ctx.llvm_ctx_ptr = LLVMAddGlobal(
                ctx.llvm_module,
                LLVMInt64TypeInContext(context),
                b"__context\0".as_ptr() as *const _,
            );
            ctx.self_ptr = &*ctx as *const Context;

            ctx.add_fn("__global_get", global_get as *mut _, 2);
            // ctx.add_fn("__global_get_func", global_get_func as *mut _, 2);
            ctx.add_fn("__global_set", global_set as *mut _, 3);
            ctx.add_fn("__add", add as *mut _, 2);
            ctx.add_fn("__sub", sub as *mut _, 2);
            ctx.add_fn("__mul", mul as *mut _, 2);
            ctx.add_fn("__div", div as *mut _, 2);
            ctx.add_fn("__array_new", array_new as *mut _, 0);
            ctx.add_fn("__array_push", array_push as *mut _, 2);
            ctx.add_fn("__string_new", string_new as *mut _, 0);
            // ctx.add_fn("__string_from", string_from as *mut _, 1);
            ctx.add_fn("__lambda_new", lambda_new as *mut _, 1);
            ctx.add_fn("__value_delete", value_delete as *mut _, 1);

            {
                let mut args = Vec::new();
                args.push(ctx.llvm_ptr);
                args.push(ctx.llvm_ptr);

                let ft = LLVMFunctionType(LLVMInt64TypeInContext(context), args.as_ptr() as *mut _, args.len() as u32, 0);
                let func = LLVMAddFunction(
                    ctx.llvm_module,
                    CString::new("__global_get_func").unwrap().as_ptr(),
                    ft,
                );

                ctx.extern_functions.insert("__global_get_func".to_string(), (
                    func,
                    global_get_func as *mut _,
                ));
            }

            {
                let mut args = Vec::new();
                args.push(LLVMPointerType(LLVMInt8TypeInContext(context), 0));

                let ft = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
                let func = LLVMAddFunction(
                    ctx.llvm_module,
                    CString::new("__string_from").unwrap().as_ptr(),
                    ft,
                );

                ctx.extern_functions.insert("__string_from".to_string(), (
                    func,
                    string_from as *mut _,
                ));
            }

            {
                let mut args = Vec::new();
                args.push(ctx.llvm_f64);

                let ft = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
                let func = LLVMAddFunction(
                    ctx.llvm_module,
                    CString::new("__float_new").unwrap().as_ptr(),
                    ft,
                );

                ctx.extern_functions.insert("__float_new".to_string(), (
                    func,
                    float_new as *mut _,
                ));
            }

            ctx
        }
    }

    pub fn read_file(&mut self, filename: &str) {
        let mut file = File::open(filename).unwrap();
        let mut source = String::new();

        file.read_to_string(&mut source).unwrap();

        let pair = RunjitParser::parse(Rule::input, &source)
            .unwrap_or_else(|e| panic!("{}", e))
            .next()
            .unwrap();

        unsafe {
            let main_func_t = LLVMFunctionType(LLVMVoidTypeInContext(self.llvm_ctx), ptr::null_mut(), 0, 0);
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

            self.block_stack.push(bb);

            LLVMPositionBuilderAtEnd(self.llvm_builder, bb);

            build::consume(self, pair);

            LLVMBuildRetVoid(self.llvm_builder);

            LLVMDisposeBuilder(self.llvm_builder);

            // let mut buffer = 0 as *mut i8;
            LLVMVerifyModule(self.llvm_module, LLVMVerifierFailureAction::LLVMAbortProcessAction, 0 as *mut _);
            // LLVMVerifyModule(self.llvm_module, LLVMVerifierFailureAction::LLVMPrintMessageAction, &mut buffer);
            // let msg = unsafe { CString::from_raw(buffer) };
            // println!("-- error --\n{:?}", msg);

            println!("-- dump --");
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

                self.runtime_variables.insert(
                    CString::new(name.as_bytes()).unwrap(),
                    Rc::new(Value::Lambda(func as u64))
                );
            }

            let addr = LLVMGetFunctionAddress(ee, b"__main__\0".as_ptr() as *const _);

            let f: extern "C" fn() = mem::transmute(addr);

            f();

            LLVMDisposeExecutionEngine(ee);
        }
    }

    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.runtime_variables
            .get(&CString::new(name).unwrap())
            .and_then(|x| if let Value::Float(f) = **x {
                Some(f)
            } else {
                None
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
