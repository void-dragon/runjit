extern crate runjit;

use std::rc::Rc;

fn main() {
    let ast = runjit::parser::read_file("samples/one.js");

    let ctx = runjit::executor::Context::new();

    ctx.set("print", runjit::executor::RustCall::new(|a| {
        for v in a {
            match **v {
                runjit::executor::Value::String(ref s) => print!("{} ", s),
                runjit::executor::Value::Float(ref s) => print!("{} ", s),
                _ => {}
            }
        }
        print!("\n");

        Ok(Rc::new(runjit::executor::Value::Null))
    }));

    if let Err(err) = runjit::executor::run(ctx, ast) {
        println!("Error: {}", err);
    }
}
