extern crate clap;
extern crate runjit;
#[macro_use]
extern crate logging;

use clap::{Arg, App};

use runjit::jit::Context;

unsafe fn myprint(val: *const runjit::jit::Value) {
    println!("in runjit {:?}", *val);
}

fn main() {
    let matches = App::new("runjit - cli")
        .version("0.1")
        .arg(Arg::with_name("file").required(true))
        .get_matches();

    let filename = matches.value_of("file").unwrap();

    let root = logging::root();
    root.clear_handlers();
    root.add_handler(logging::FileHandler::new("cli.log", true));

    debug!("start");

    let mut ctx = Context::new();

    ctx.add_fn("print", myprint as *mut _, 1);

    debug!("--- read ---");

    ctx.read_file(filename);

    debug!("--- run ---");

    ctx.run();

    debug!("{:?}", ctx.get("myvar"));
    debug!("{:?}", ctx.get("x"));
    debug!("{:?}", ctx.get("arr"));
    debug!("--- done ---");
}
