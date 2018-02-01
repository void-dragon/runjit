//!
//! AST executor
//!
use std::collections::BTreeMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use ast::*;
use types::*;



pub struct Context {
    pub parent: Option<Rc<Context>>,
    pub values: RefCell<BTreeMap<String, Rc<Value>>>,
    pub statements: Vec<Ast>,
}

impl Context {
    pub fn new() -> Rc<Context> {
        Rc::new(Context {
            parent: None,
            values: RefCell::new(BTreeMap::new()),
            statements: Vec::new(),
        })
    }

    pub fn with_parent(parent: Rc<Context>) -> Rc<Context> {
        Rc::new(Context {
            parent: Some(parent),
            values: RefCell::new(BTreeMap::new()),
            statements: Vec::new(),
        })
    }

    pub fn get(&self, name: &str) -> Option<Rc<Value>> {
        let vals = self.values.borrow();
        let maybe = vals.get(name);

        match maybe {
            Some(m) => Some(m.clone()),
            None => {
                if let Some(ref parent) = self.parent {
                    parent.get(name)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_by_ast(&self, var: &Vec<Rc<Ast>>) -> Option<Rc<Value>> {
        for i in var {
            match **i {
                Ast::Str(ref name) => {}
                Ast::Exp(_, _, _) => {}
                _ => {}
            }
        }

        None
    }

    pub fn set(&self, name: &str, val: Rc<Value>) {
        let mut vals = self.values.borrow_mut();
        vals.insert(name.to_string(), val);
    }

    pub fn set_by_ast(&self, var: &Vec<Rc<Ast>>, val: Rc<Value>) {}
}


pub fn run(ctx: Rc<Context>, ast: Rc<Ast>) -> Result<Rc<Value>, String> {
    match ast.deref() {
        &Ast::Block(ref b) => block(ctx.clone(), b),
        _ => Err(String::from("unexpected ast element")),
    }
}

fn block(ctx: Rc<Context>, data: &Vec<Rc<Ast>>) -> Result<Rc<Value>, String> {
    for stmnt in data {
        let res = match stmnt.deref() {
            &Ast::Assign(ref name, ref ast) => assign(ctx.clone(), name.clone(), ast.clone()),
            &Ast::Call(ref name, ref ast) => call(ctx.clone(), name, ast),
            &Ast::If(ref exp, ref block, ref _else) => {
                _if(ctx.clone(), exp.clone(), &block, _else.clone())
            }
            _ => Err(String::from("unexpected ast element")),
        };

        if res.is_err() {
            return res;
        }
    }

    Ok(Rc::new(Value::Null))
}

fn exp(ctx: Rc<Context>, ast: Rc<Ast>) -> Result<Rc<Value>, String> {
    match ast.deref() {
        &Ast::Str(ref data) => Ok(Rc::new(Value::String(data.clone()))),
        &Ast::Float(ref data) => Ok(Rc::new(Value::Float(*data))),
        &Ast::Lambda(ref params, ref stmnts) => {
            Ok(Rc::new(Value::Lambda(params.clone(), stmnts.clone())))
        }
        &Ast::Var(ref tokens) => {
            ctx.get_by_ast(tokens).ok_or(
                String::from("unknown variable"),
            )
        }
        &Ast::Exp(ref op, ref left, ref right) => {
            let l = exp(ctx.clone(), left.clone());
            let r = exp(ctx.clone(), right.clone());

            match l {
                Ok(v) => {
                    if let Value::Float(lf) = *v {
                        match r {
                            Ok(v) => {
                                if let Value::Float(rf) = *v {
                                    match op {
                                        &Operation::Add => Ok(Rc::new(Value::Float(lf + rf))),
                                        &Operation::Sub => Ok(Rc::new(Value::Float(lf - rf))),
                                        &Operation::Mul => Ok(Rc::new(Value::Float(lf * rf))),
                                        &Operation::Div => Ok(Rc::new(Value::Float(lf / rf))),
                                        &Operation::Mod => Ok(Rc::new(Value::Float(lf % rf))),
                                        &Operation::Eq => {
                                            if lf == rf {
                                                Ok(Rc::new(Value::Float(1.0)))
                                            } else {
                                                Ok(Rc::new(Value::Null))
                                            }
                                        }
                                        &Operation::Neq => {
                                            if lf != rf {
                                                Ok(Rc::new(Value::Float(1.0)))
                                            } else {
                                                Ok(Rc::new(Value::Null))
                                            }
                                        }
                                        _ => Err(String::from("unsupported operation")),
                                    }
                                } else {
                                    Err(String::from("only can calculate numbers"))
                                }
                            }
                            Err(e) => Err(e),
                        }
                    } else {
                        Err(String::from("only can calculate numbers"))
                    }
                }
                Err(e) => Err(e),
            }
        }
        _ => Err(format!("unexpected expression")),
    }
}

fn assign(ctx: Rc<Context>, name: Rc<Ast>, ast: Rc<Ast>) -> Result<Rc<Value>, String> {
    if let Ok(val) = exp(ctx.clone(), ast) {
        if let Ast::Var(ref tokens) = *name {
            ctx.set_by_ast(tokens, val);
        }
    }

    Ok(Rc::new(Value::Null))
}

fn call(ctx: Rc<Context>, name: &str, ast: &Vec<Rc<Ast>>) -> Result<Rc<Value>, String> {
    let maybe = ctx.get(name);

    if let Some(val) = maybe {
        let params: Vec<Rc<Value>> = ast.iter()
            .map(|x| exp(ctx.clone(), x.clone()).unwrap())
            .collect();

        match *val {
            Value::Lambda(ref names, ref stmnts) => {
                let new_ctx = Context::with_parent(ctx);

                for i in 0..params.len() {
                    new_ctx.set(&names[i], params[i].clone());
                }

                block(new_ctx, stmnts)
            }
            Value::RustCall(ref rc) => rc.call(&params),
            _ => Err(String::from("unexpected value")),
        }
    } else {
        Err(String::from("unknown call of variable"))
    }
}

fn _if(
    ctx: Rc<Context>,
    ex: Rc<Ast>,
    blck: &Vec<Rc<Ast>>,
    el: Rc<Ast>,
) -> Result<Rc<Value>, String> {
    let res = exp(ctx.clone(), ex);

    match res {
        Ok(r) => {
            match *r {
                Value::Null => Ok(Rc::new(Value::Null)),
                _ => {
                    let new_ctx = Context::with_parent(ctx);

                    block(new_ctx, blck)
                }
            }
        }
        Err(e) => Err(e),
    }
}
