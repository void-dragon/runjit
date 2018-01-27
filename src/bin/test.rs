extern crate runjit;

use runjit::jit::Context;

fn myprint() {
    println!("in runjit");
}

fn main() {
    let mut ctx = Context::new();

    ctx.add_fn("myprint", myprint as *mut _, 0);

    ctx.read_file("samples/one.js");

    ctx.run();

    println!("{:?}", ctx.get_float("myvar"));
}
