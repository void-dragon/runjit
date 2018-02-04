extern crate runjit;

use runjit::jit::Context;

fn myprint() {
    println!("in runjit");
}

fn main() {
    let mut ctx = Context::new();

    ctx.add_fn("print", myprint as *mut _, 0);

    println!("--- read ---");

    ctx.read_file("samples/one.js");

    println!("--- run ---");

    ctx.run();

    println!("{:?}", ctx.get("myvar"));
    println!("{:?}", ctx.get("x"));
    println!("{:?}", ctx.get("arr"));
    println!("--- done ---");
}
