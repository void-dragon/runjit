
use std::rc::Rc;

use ast::Ast;

pub enum Value {
    Float(f64),
    String(String),
    Lambda(Vec<String>, Vec<Rc<Ast>>),
    RustCall(Box<Call>),
    Null,
}

pub trait Dict {}

type Args = Vec<Rc<Value>>;

pub trait Call {
    fn call(&self, args: &Args) -> Result<Rc<Value>, String>;
}

pub struct RustCall<T>
where
    T: Fn(&Args) -> Result<Rc<Value>, String>,
{
    func: T,
}

impl<T> RustCall<T>
where
    T: 'static,
    T: Fn(&Args) -> Result<Rc<Value>, String>,
{
    pub fn new(f: T) -> Rc<Value> {
        Rc::new(Value::RustCall(Box::new(RustCall { func: f })))
    }
}

impl<T> Call for RustCall<T>
where
    T: Fn(&Args) -> Result<Rc<Value>, String>,
{
    fn call(&self, args: &Args) -> Result<Rc<Value>, String> {
        // println!("arg count: {}", args.len());
        (self.func)(args)
    }
}
