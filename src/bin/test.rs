extern crate runjit;

use runjit::jit::Context;

fn myprint() {
    println!("in runjit");
}

fn main() {
    let mut ctx = Context::new();

    ctx.add_fn("myprint", myprint as *mut _);

    ctx.read_file("samples/one.js");

    ctx.run();
}
