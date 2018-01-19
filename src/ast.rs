
use std::rc::Rc;

#[derive(Debug)]
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Neq,
    Gt,
    Le,
    Gte,
    Lee,
}

#[derive(Debug)]
pub enum Ast {
    Exp(Operation, Rc<Ast>, Rc<Ast>),
    Float(f64),
    Str(String),
    Var(String),
    Lambda(Vec<String>, Vec<Rc<Ast>>),
    Call(String, Vec<Rc<Ast>>),
    Assign(String, Rc<Ast>),
    If(Rc<Ast>, Vec<Rc<Ast>>, Rc<Ast>),
    Loop(Rc<Ast>, Vec<Rc<Ast>>),
    Block(Vec<Rc<Ast>>),
    Nothing,
}

