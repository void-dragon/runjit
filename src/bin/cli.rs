extern crate clap;
extern crate runjit;

#[macro_use]
extern crate log;

use clap::{Arg, App};
use std::io::Write;
use std::sync::Mutex;

use runjit::jit::Context;

unsafe fn myprint(val: *const runjit::jit::Value) {
    println!("in runjit {:?}", *val);
}

struct FileLogger {
    out: Option<Mutex<std::fs::File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            if let Some(ref out) = self.out {
                writeln!(*out.lock().unwrap(), "{} | {} | {:?}:{:?} | {}", record.level(), record.target(), record.file().unwrap(), record.line().unwrap(), record.args()).unwrap();
            }
        }
    }

    fn flush(&self) {}
}

static mut LOGGER: FileLogger = FileLogger { out: None };

fn main() {
    let matches = App::new("runjit - cli")
        .version("0.1")
        .arg(Arg::with_name("file").required(true))
        .get_matches();

    let filename = matches.value_of("file").unwrap();

    unsafe {
        LOGGER.out = Some(Mutex::new(std::fs::File::create("cli.log").unwrap()));
        log::set_logger(&LOGGER).unwrap();
    }
    log::set_max_level(log::LevelFilter::Debug);

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
