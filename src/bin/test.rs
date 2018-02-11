extern crate runjit;
#[macro_use]
extern crate logging;

use runjit::jit::Context;

unsafe fn myprint(val: *const runjit::jit::Value) {
    println!("in runjit {:?}", *val);
}

fn main() {
    let root = logging::root();
    root.clear_handlers();
    root.add_handler(logging::FileHandler::new("test.log", true));

    debug!("start");

    let mut ctx = Context::new();

    ctx.add_fn("print", myprint as *mut _, 1);

    debug!("--- read ---");

    ctx.read_file("samples/one.js");

    debug!("--- run ---");

    ctx.run();

    debug!("{:?}", ctx.get("myvar"));
    debug!("{:?}", ctx.get("x"));
    debug!("{:?}", ctx.get("arr"));
    debug!("--- done ---");
}
