
use pest::Parser;
use pest::inputs::{Input, FileInput};
use pest::iterators::Pair;

use std::rc::Rc;

use ast::*;

#[derive(Parser)]
#[grammar = "runjit.pest"]
struct RunjitParser;

//+
fn consume<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    match pair.as_rule() {
        Rule::block => block(pair),
        Rule::statement => statement(pair),
        _ => panic!("unexpected token"),
    }
}

fn block<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let mut v = Vec::new();

    for pair in pair.into_inner() {
        v.push(consume(pair));
    }

    Rc::new(Ast::Block(v))
}

fn statement<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let next = pair.into_inner().next().unwrap();

    match next.as_rule() {
        Rule::assign => assign(next),
        Rule::call => call(next),
        Rule::_if => _if(next),
        _ => panic!("unrecognized statement: {:?}", next.as_rule()),
    }
}

fn exp<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let mut inner = pair.into_inner();

    let next = inner.next().unwrap();
    let rule = next.as_rule();
    let left = match rule {
        Rule::exp => exp(next),
        Rule::literal => {
            let inner = next.into_inner().next().unwrap();

            match inner.as_rule() {
                Rule::numeric => {
                    let s = inner.as_str().trim();
                    
                    let d: f64 = if let Ok(x) = s.parse() {
                        x
                    } else {
                        s.parse::<i64>().unwrap() as f64
                    };

                    Rc::new(Ast::Float(d))
                } 
                _ => Rc::new(Ast::Str(String::from(inner.as_str()))),
            }
        }
        Rule::ident => Rc::new(Ast::Var(String::from(next.as_str()))),
        _ => panic!("unknown exp: {:?}", rule),
    };

    if let Some(op) = inner.next() {
        if let Some(right) = inner.next() {

            let the_op = match op.as_rule() {
                Rule::op_add => Operation::Add,
                Rule::op_sub => Operation::Sub,
                Rule::op_mul => Operation::Mul,
                Rule::op_div => Operation::Div,
                Rule::op_mod => Operation::Mod,
                Rule::op_and => Operation::And,
                Rule::op_or => Operation::Or,
                Rule::op_eq => Operation::Eq,
                Rule::op_neq => Operation::Neq,
                Rule::op_gt => Operation::Gt,
                Rule::op_le => Operation::Le,
                Rule::op_gte => Operation::Gte,
                Rule::op_lee => Operation::Lee,
                _ => panic!("unknown operation in expression: {:?}", op.as_rule()),
            };

            Rc::new(Ast::Exp(the_op, left, exp(right)))
        } else {
            panic!("incomplete expression")
        }
    } else {
        left
    }

}

fn assign<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let mut inner = pair.into_inner();

    let ident = String::from(inner.next().unwrap().as_str());
    let e = inner.next().unwrap();

    let ex = match e.as_rule() {
        Rule::exp => exp(e),
        Rule::lambda => lambda(e),
        Rule::dict => dict(e),
        Rule::array => array(e),
        _ => panic!("unexpected assign: {:?}", e.as_rule()),
    };

    Rc::new(Ast::Assign(ident, ex))
}

fn lambda<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let inner = pair.into_inner();
    let mut names = Vec::new();
    let mut statements = Vec::new();

    for node in inner {
        match node.as_rule() {
            Rule::names => {
                let inner = node.into_inner();

                for node in inner {
                    names.push(String::from(node.as_str()));
                }
            }
            Rule::block => {
                if let Ok(Ast::Block(stmnts)) = Rc::try_unwrap(block(node)) {
                    statements = stmnts;
                }
            }
            _ => panic!("unexpected element"),
        }
    }

    Rc::new(Ast::Lambda(names, statements))
}

fn call<I: Input>(pair:  Pair<Rule, I> ) -> Rc<Ast> {
    let mut call = pair.into_inner();

    let name = String::from(call.next().unwrap().as_str());
    
    let mut params = Vec::new();
    
    if let Some(ps) = call.next() {
        for mut param in ps.into_inner() {
            let val = match param.as_rule() {
                Rule::exp => params.push(exp(param)),
                _ => panic!("unexpected stuff"),
            };
        }
    }

    Rc::new(Ast::Call(name, params))
}

fn _if<I: Input>(pair:  Pair<Rule, I>) -> Rc<Ast> {
    let mut the_if = pair.into_inner();

    let cond = exp(the_if.next().unwrap());
    let block = if let Ok(Ast::Block(b)) = Rc::try_unwrap(block(the_if.next().unwrap())) {
        b
    } else {
        Vec::new()
    };
    let elsy = match the_if.next() {
        Some(block) => _else(block),
        None => Rc::new(Ast::Nothing),
    };
    
    Rc::new(Ast::If(cond, block, elsy))
}

fn _else<I: Input>(pair:  Pair<Rule, I>) -> Rc<Ast> {
    let mut the_else = pair.into_inner();

    Rc::new(Ast::Nothing)
}

fn dict<I: Input>(pair:  Pair<Rule, I>) -> Rc<Ast> {
    let mut the_else = pair.into_inner();

    Rc::new(Ast::Nothing)
}

fn array<I: Input>(pair:  Pair<Rule, I>) -> Rc<Ast> {
    let mut the_else = pair.into_inner();

    Rc::new(Ast::Nothing)
}

pub fn read_file(filename: &str) -> Rc<Ast> {
    let source = FileInput::new(filename).unwrap();

    let pair = RunjitParser::parse(Rule::input, Rc::new(source))
        .unwrap_or_else(|e| panic!("{}", e))
        .next()
        .unwrap();
    
    consume(pair)
}
