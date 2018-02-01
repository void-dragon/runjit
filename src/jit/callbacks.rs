//!
//! Nice and fluffy rust callbacks for the llvm generated code.
//!
use std::ffi::{CStr, CString};
use std::rc::Rc;

use libc;

use jit::{Context, Value};

pub unsafe extern "C" fn global_get(ctx: *mut Context, name: *const Value) -> *const Value {
    println!("!! get {:?} !!", *name);

    if let Value::Array(ref a) = *name {
        if let Value::Str(ref s) = *a[0] {
            let val = (*ctx).runtime_variables.get(s);

            if let Some(val) = val {
                return Rc::into_raw(val.clone());
            }
        }
    }

    &Value::Null as *const _
}

pub unsafe extern "C" fn global_get_func(ctx: *mut Context, name: *const Value) -> u64 {
    println!("!! get func {:?} !!", *name);

    if let Value::Array(ref a) = *name {
        if let Value::Str(ref s) = *a[0] {
            let val = (*ctx).runtime_variables.get(s);

            if let Value::Lambda(v) = **val.unwrap() {
                return v;
            }
        }
    }

    0
}

pub unsafe extern "C" fn global_set(
    ctx: *mut Context,
    name: *const Value,
    val: *mut Value,
) -> *const Value {
    println!("!! set {:?} = {:?} !!", *name, *val);

    if let Value::Array(ref a) = *name {
        if let Value::Str(ref s) = *a[0] {
            (*ctx).runtime_variables.insert(
                s.clone(),
                Rc::from_raw(val),
            );
        }
    }

    &Value::Null as *const _
}

pub unsafe extern "C" fn add(left: *const Value, right: *const Value) -> *const Value {
    println!("!! add !!");

    let left_rc = Rc::from_raw(left);
    let right_rc = Rc::from_raw(right);

    if let Value::Float(l) = *left_rc {
        if let Value::Float(r) = *right_rc {
            return Rc::into_raw(Rc::new(Value::Float(l + r)));
        }
    }

    Rc::into_raw(Rc::new(Value::Float(0.0)))
}

pub unsafe extern "C" fn sub(left: *const Value, right: *const Value) -> *const Value {
    println!("!! sub !!");

    let left_rc = Rc::from_raw(left);
    let right_rc = Rc::from_raw(right);

    if let Value::Float(l) = *left_rc {
        if let Value::Float(r) = *right_rc {
            return Rc::into_raw(Rc::new(Value::Float(l - r)));
        }
    }

    Rc::into_raw(Rc::new(Value::Float(0.0)))
}

pub unsafe extern "C" fn mul(left: *const Value, right: *const Value) -> *const Value {
    println!("!! mul !!");

    let left_rc = Rc::from_raw(left);
    let right_rc = Rc::from_raw(right);

    if let Value::Float(l) = *left_rc {
        if let Value::Float(r) = *right_rc {
            return Rc::into_raw(Rc::new(Value::Float(l * r)));
        }
    }

    Rc::into_raw(Rc::new(Value::Float(0.0)))
}

pub unsafe extern "C" fn div(left: *const Value, right: *const Value) -> *const Value {
    println!("!! div !!");

    let left_rc = Rc::from_raw(left);
    let right_rc = Rc::from_raw(right);

    if let Value::Float(l) = *left_rc {
        if let Value::Float(r) = *right_rc {
            return Rc::into_raw(Rc::new(Value::Float(l / r)));
        }
    }

    Rc::into_raw(Rc::new(Value::Float(0.0)))
}

pub extern "C" fn array_new() -> *const Value {
    println!("!! new array !!");
    Rc::into_raw(Rc::new(Value::Array(Vec::new())))
}

pub unsafe extern "C" fn array_push(arr: *mut Value, v: *mut Value) -> *const Value {
    println!("!! pushing value !! {:?} {:?}", *arr, *v);

    if let Value::Array(ref mut a) = *arr {
        a.push(Rc::from_raw(v));
    }

    &Value::Null as *const _
}

pub extern "C" fn string_new() -> *const Value {
    println!("!! new string !!");
    Rc::into_raw(Rc::new(Value::Str(CString::new("").unwrap())))
}

pub unsafe extern "C" fn string_from(bytes: *mut libc::c_char) -> *const Value {
    println!("!! string from !!");
    // let data = CString::from_raw(bytes);
    let data = CStr::from_ptr(bytes);
    Rc::into_raw(Rc::new(Value::Str(data.to_owned())))
}

pub extern "C" fn float_new(v: f64) -> *const Value {
    println!("!! new float {} !!", v);
    Rc::into_raw(Rc::new(Value::Float(v)))
}

pub extern "C" fn lambda_new(v: u64) -> *const Value {
    println!("!! new lambda {} !!", v);
    Rc::into_raw(Rc::new(Value::Lambda(v)))
}

pub extern "C" fn value_delete(a: *const Value) -> *const Value {
    println!("!! delete value !!");

    unsafe { Rc::from_raw(a) };

    &Value::Null as *const _
}
