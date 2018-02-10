//!
//! Build llvm ir from parser input.
//!
//! TODO: reuse string constants
//! TODO: optemize pure expressions
//!
use pest::iterators::Pair;

use logging;

use llvm;
use llvm::prelude::*;
use llvm::core::*;

use std::collections::BTreeMap;
use std::ffi::CString;

use parser::*;

use jit::Context;

pub fn consume(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("consume");

    match pair.as_rule() {
        Rule::block => block(ctx, pair),
        Rule::statement => statement(ctx, pair),
        _ => panic!("unexpected token"),
    }
}

fn block(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("block");
    let mut last = 0 as LLVMValueRef;

    for pair in pair.into_inner() {
        last = consume(ctx, pair);
    }

    last
}

fn statement(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("statement");
    let next = pair.into_inner().next().unwrap();

    match next.as_rule() {
        Rule::assign => assign(ctx, next),
        Rule::call => call(ctx, next),
        //     // Rule::_if => _if(next),
        _ => panic!("unrecognized statement: {:?}", next.as_rule()),
    }
}

unsafe fn build_string(ctx: &mut Context, s: &str) -> LLVMValueRef {
    logging::debug("build_string");
    let cstr = LLVMBuildGlobalStringPtr(
        ctx.llvm_builder,
        CString::new(s).unwrap().as_ptr(),
        b"__str\0".as_ptr() as *const _,
    );

    let args = vec![cstr];
    let string_from = ctx.extern_functions.get("__string_from").unwrap();
    LLVMBuildCall(
        ctx.llvm_builder,
        string_from.0,
        args.as_ptr() as *mut LLVMValueRef,
        args.len() as u32,
        b"__string_from\0".as_ptr() as *const _,
    )
}

fn string(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("string");
    let s = pair.as_str();
    unsafe { build_string(ctx, s) }
}

enum AccessToken {
    Pure(String),
    Parts(Vec<LLVMValueRef>),
}

fn access(ctx: &mut Context, pair: Pair<Rule>) -> AccessToken {
    logging::debug("access");
    let inner: Vec<_> = pair.into_inner().collect();

    if inner.len() == 1 {
        let name = inner[0].as_str().trim().to_string();
        AccessToken::Pure(name)
    } else {
        let mut parts = Vec::new();

        for p in inner {
            let x = match p.as_rule() {
                Rule::ident => string(ctx, p),
                Rule::exp => unsafe { exp(ctx, p) },
                _ => panic!("unexpected in access rule"),
            };

            parts.push(x);
        }

        AccessToken::Parts(parts)
    }
}

fn build_access_array(ctx: &mut Context, parts: &Vec<LLVMValueRef>) -> LLVMValueRef {
    logging::debug("build_access_array");
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

    let array_push = ctx.extern_functions.get("__array_push").unwrap().clone();

    for p in parts {
        let args = vec![an_ref, *p];
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

    an_ref
}

fn build_global_get(ctx: &mut Context, name: LLVMValueRef) -> LLVMValueRef {
    logging::debug("build_global_get");
    let get_global = ctx.extern_functions.get("__global_get_func").unwrap();
    let value_delete = ctx.extern_functions.get("__value_delete").unwrap();
    let args = vec![ctx.llvm_ctx_ptr, name];
    let delete_args = vec![name];

    unsafe {
        let func = LLVMBuildCall(
            ctx.llvm_builder,
            get_global.0,
            args.as_ptr() as *mut _,
            args.len() as u32,
            b"global_get_func\0".as_ptr() as *const _,
        );

        LLVMBuildCall(
            ctx.llvm_builder,
            value_delete.0,
            delete_args.as_ptr() as *mut LLVMValueRef,
            delete_args.len() as u32,
            b"__value_delete\0".as_ptr() as *const _,
        );

        func
    }
}

fn build_global_set(ctx: &mut Context, name: LLVMValueRef, value: LLVMValueRef) -> LLVMValueRef {
    logging::debug("build_global_set");
    let global_set = ctx.extern_functions.get("__global_set").unwrap();
    let value_delete = ctx.extern_functions.get("__value_delete").unwrap();
    let args = vec![ctx.llvm_ctx_ptr, name, value];
    let delete_args = vec![name];

    unsafe {
        let ret = LLVMBuildCall(
            ctx.llvm_builder,
            global_set.0,
            args.as_ptr() as *mut LLVMValueRef,
            args.len() as u32,
            b"__global_set\0".as_ptr() as *const _,
        );

        LLVMBuildCall(
            ctx.llvm_builder,
            value_delete.0,
            delete_args.as_ptr() as *mut LLVMValueRef,
            delete_args.len() as u32,
            b"__value_delete\0".as_ptr() as *const _,
        );

        ret
    }
}

unsafe fn const_op(ctx: &mut Context, left_ref: LLVMValueRef, right_ref: LLVMValueRef, op: Pair<Rule>) -> LLVMValueRef {
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
}

unsafe fn generic_op(ctx: &mut Context, left_ref: LLVMValueRef, right_ref: LLVMValueRef, op: Pair<Rule>) -> LLVMValueRef {
    logging::debug("generic_op");
    let call = match op.as_rule() {
        Rule::op_add => ctx.extern_functions.get("__add").unwrap(),
        Rule::op_sub => ctx.extern_functions.get("__sub").unwrap(),
        Rule::op_mul => ctx.extern_functions.get("__mul").unwrap(),
        Rule::op_div => ctx.extern_functions.get("__div").unwrap(),
        // Rule::op_mod => Operation::Mod,
        Rule::op_and => ctx.extern_functions.get("__and").unwrap(),
        Rule::op_or => ctx.extern_functions.get("__or").unwrap(),
        Rule::op_eq => ctx.extern_functions.get("__eq").unwrap(),
        Rule::op_neq => ctx.extern_functions.get("__neq").unwrap(),
        Rule::op_gt => ctx.extern_functions.get("__gt").unwrap(),
        Rule::op_le => ctx.extern_functions.get("__le").unwrap(),
        Rule::op_gte => ctx.extern_functions.get("__gte").unwrap(),
        Rule::op_lee => ctx.extern_functions.get("__lee").unwrap(),
        _ => panic!("unknown operation in expression: {:?}", op.as_rule()),
    };

    let args = vec![left_ref, right_ref];
    LLVMBuildCall(
        ctx.llvm_builder,
        call.0,
        args.as_ptr() as *mut LLVMValueRef,
        args.len() as u32,
        b"__op_res\0".as_ptr() as *const _,
    )
}

unsafe fn exp(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("exp");
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
                Rule::string_literal => {
                    let s = inner.as_str();
                    build_string(ctx, s.slice_unchecked(1, s.len() - 1))
                }
                _ => panic!("not supported yet"),
            }
        }
        Rule::access => {
            match access(ctx, next) {
                AccessToken::Pure(name) => {
                    *ctx.local_stack.last().unwrap().get(&name).unwrap()
                }
                AccessToken::Parts(parts) => {
                    let name = build_access_array(ctx, &parts);
                    build_global_get(ctx, name)
                }
            }
        }
        _ => panic!("unknown exp: {:?}", rule),
    };

    if let Some(op) = inner.next() {
        if let Some(right) = inner.next() {
            let right_ref = exp(ctx, right);
            // TODO: Use more const op, to make things FAAAAAST!!!
            // const_op(ctx, left_ref, right_ref, op)
            generic_op(ctx, left_ref, right_ref, op)
        } else {
            panic!("incomplete expression")
        }
    } else {
        left_ref
    }

}

fn assign(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("assign");
    let mut inner = pair.into_inner();

    let v = inner.next().unwrap();
    let access_token = match v.as_rule() {
        Rule::access => access(ctx, v),
        _ => panic!("expected access"),
    };

    let e = inner.next().unwrap();
    let ex = match e.as_rule() {
        Rule::exp => unsafe { exp(ctx, e) }
        Rule::lambda => lambda(ctx, e),
        Rule::dict => dict(ctx, e),
        Rule::array => array(ctx, e),
        _ => panic!("unexpected assign: {:?}", e.as_rule()),
    };

    match access_token {
        AccessToken::Pure(name) => {
            logging::debug("  pure");
            if ctx.local_stack.len() > 0 {
                ctx.local_stack.last_mut().unwrap().insert(name, ex);

                ex
            } else {
                let nurf = unsafe { build_string(ctx, &name) };
                let parts = vec![nurf];
                let ident_array = build_access_array(ctx, &parts);
                build_global_set(ctx, ident_array, ex)
            }
        }
        AccessToken::Parts(parts) => {
            logging::debug("  parts");
            let ident_array = build_access_array(ctx, &parts);
            build_global_set(ctx, ident_array, ex)
        }
    }
}

fn lambda(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("lambda");
    let mut inner = pair.into_inner();
    let mut params = Vec::new();
    let mut param_refs = BTreeMap::new();
    let mut args = Vec::new();

    for node in inner.next().unwrap().into_inner() {
        params.push(String::from(node.as_str()));
        args.push(ctx.llvm_ptr);
    }

    unsafe {
        let ftype = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
        let func = LLVMAddFunction(ctx.llvm_module, b"__lambda\0".as_ptr() as *const _, ftype);

        for i in 0..params.len() {
            let val = LLVMGetParam(func, i as u32);
            param_refs.insert(params[i].clone(), val);
        }

        let bb = LLVMAppendBasicBlockInContext(ctx.llvm_ctx, func, b"__entry\0".as_ptr() as *const _ );
        LLVMPositionBuilderAtEnd(ctx.llvm_builder, bb);

        ctx.local_stack.push(param_refs);
        ctx.block_stack.push(bb);

        let last = block(ctx, inner.next().unwrap());

        logging::debug("  build ret");

        LLVMBuildRet(ctx.llvm_builder, last);

        ctx.block_stack.pop();
        ctx.local_stack.pop();
        LLVMPositionBuilderAtEnd(ctx.llvm_builder, ctx.block_stack[ctx.block_stack.len() - 1]);

        logging::debug(&format!("  cast func ptr {:?} {:?}", LLVMGetValueKind(func), LLVMGetTypeKind(ctx.llvm_ptr)));

        // let ptr = LLVMBuildPtrToInt(ctx.llvm_builder, func, ctx.llvm_ptr, b"__lambda_address\0".as_ptr() as *const _);
        let ptr = LLVMBuildBitCast(ctx.llvm_builder, func, ctx.llvm_ptr, b"__lambda_address\0".as_ptr() as *const _);
        let lambda_new = ctx.extern_functions.get("__lambda_new").unwrap();
        let args = vec![ptr];

        logging::debug("  build new lambda value");

        LLVMBuildCall(
            ctx.llvm_builder,
            lambda_new.0,
            args.as_ptr() as *mut LLVMValueRef,
            args.len() as u32,
            b"__lambda_address\0".as_ptr() as *const _,
        )
    }

}

fn call(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("call");
    let mut call = pair.into_inner();
    let mut params = Vec::new();

    let access_token = access(ctx, call.next().unwrap());

    logging::debug("  params");

    if let Some(ps) = call.next() {
        for mut param in ps.into_inner() {
            match param.as_rule() {
                Rule::exp => params.push(unsafe { exp(ctx, param) } ),
                _ => panic!("unexpected rule: {:?}", param.as_rule()),
            }
        }
    }

    let func_ptr = match access_token {
        AccessToken::Pure(name) => {
            logging::debug("  pure");
            let val = ctx.local_stack.last().and_then(|v| match v.get(&name) {
                Some(var) => Some(*var),
                None => {
                    let val = { logging::debug(&format!("  extern func {}", name)); ctx.extern_functions.get(&name).clone() };
                    match val {
                        Some(efunc) => Some(efunc.0),
                        None => {
                            None
                        }
                    }
                }
            });

            match val {
                Some(v) => v,
                None => {
                    let val = unsafe { build_string(ctx, &name) };
                    let name = build_access_array(ctx, &vec![val]);
                    let func = build_global_get(ctx, name);

                    unsafe {
                        let ptr_type = LLVMPointerType(LLVMFunctionType(ctx.llvm_ptr, params.as_ptr() as *mut _, params.len() as u32, 0), 0);
                        // let ptr_type = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
                        let func_ptr = LLVMBuildIntToPtr(ctx.llvm_builder, func, ptr_type, b"var_to_func\0".as_ptr() as *const _);
                        // let func_ptr = LLVMBuildBitCast(ctx.llvm_builder, func, ptr_type, b"var_to_func\0".as_ptr() as *const _);

                        func_ptr
                    }
                }
            }
        }
        AccessToken::Parts(parts) => {
            logging::debug("  parts");
            let name = build_access_array(ctx, &parts);
            let func = build_global_get(ctx, name);

            unsafe {
                let ptr_type = LLVMPointerType(LLVMFunctionType(ctx.llvm_ptr, params.as_ptr() as *mut _, params.len() as u32, 0), 0);
                // let ptr_type = LLVMFunctionType(ctx.llvm_ptr, args.as_ptr() as *mut _, args.len() as u32, 0);
                let func_ptr = LLVMBuildIntToPtr(ctx.llvm_builder, func, ptr_type, b"var_to_func\0".as_ptr() as *const _);
                // let func_ptr = LLVMBuildBitCast(ctx.llvm_builder, func, ptr_type, b"var_to_func\0".as_ptr() as *const _);

                func_ptr
            }
        }
    };

    logging::debug(&format!("  build {:?}", unsafe { LLVMGetValueKind(func_ptr) }));
    unsafe {
        LLVMBuildCall(
            ctx.llvm_builder,
            func_ptr,
            params.as_ptr() as *mut _,
            params.len() as u32,
            b"call\0".as_ptr() as *const _,
        )
    }
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

fn dict(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("dict");
    let inner = pair.into_inner();
    let dict_new = ctx.extern_functions.get("__dict_new").unwrap().clone();
    let dict_insert = ctx.extern_functions.get("__dict_insert").unwrap().clone();

    let dct = unsafe {
        LLVMBuildCall(
            ctx.llvm_builder,
            dict_new.0,
            0 as *mut LLVMValueRef,
            0,
            b"__dict_new\0".as_ptr() as *const _,
        )
    };

    for p in inner {
        let mut inner = p.into_inner();

        unsafe {
            let ident = string(ctx, inner.next().unwrap());
            let args = vec![dct, ident, exp(ctx, inner.next().unwrap())];

            LLVMBuildCall(
                ctx.llvm_builder,
                dict_insert.0,
                args.as_ptr() as *mut LLVMValueRef,
                args.len() as u32,
                b"__dict_insert\0".as_ptr() as *const _,
            );
        }
    }

    dct
}

fn array(ctx: &mut Context, pair: Pair<Rule>) -> LLVMValueRef {
    logging::debug("array");
    let inner = pair.into_inner();
    let array_new = ctx.extern_functions.get("__array_new").unwrap().clone();
    let array_push = ctx.extern_functions.get("__array_push").unwrap().clone();

    let arr = unsafe {
        LLVMBuildCall(
            ctx.llvm_builder,
            array_new.0,
            0 as *mut LLVMValueRef,
            0,
            b"__array_new\0".as_ptr() as *const _,
        )
    };

    for p in inner {
        unsafe {
            let args = vec![arr, exp(ctx, p)];

            LLVMBuildCall(
                ctx.llvm_builder,
                array_push.0,
                args.as_ptr() as *mut LLVMValueRef,
                args.len() as u32,
                b"__array_push\0".as_ptr() as *const _,
            );
        }
    }

    arr
}
