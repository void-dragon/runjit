extern crate runjit;
extern crate logging;

use runjit::jit::Context;

fn myprint() {
    println!("in runjit");
}

fn main() {
    let root = logging::root();
    root.clear_handlers();
    root.add_handler(logging::FileHandler::new("test.log"));

    logging::debug("start");

    let mut ctx = Context::new();

    ctx.add_fn("print", myprint as *mut _, 0);

    logging::debug("--- read ---");

    ctx.read_file("samples/one.js");

    logging::debug("--- run ---");

    ctx.run();

    logging::debug(&format!("{:?}", ctx.get("myvar")));
    logging::debug(&format!("{:?}", ctx.get("x")));
    logging::debug(&format!("{:?}", ctx.get("arr")));
    logging::debug(&format!("--- done ---"));
}
