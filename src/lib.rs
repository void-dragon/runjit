//!
//! **example.rj**
//!
//! ```js
//! call = (x) => { print() }
//! call()
//! ```
//!
//! **main.rs**
//!
//! ```rust
//! extern crate runjit;
//!
//! use runjit::jit::Context;
//!
//! fn myprint() {
//!     println!("in runjit");
//! }
//!
//! fn main() {
//!     let mut ctx = Context::new();
//!
//!     ctx.add_fn("print", myprint as *mut _, 0);
//!
//!     ctx.read_file("example.rj");
//!
//!     ctx.run();
//!
//!     println!("{:?}", ctx.get_float("myvar"));
//! }
//! ```
//!
extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate llvm_sys as llvm;
extern crate libc;


pub mod ast;
pub mod executor;
pub mod jit;
pub mod parser;
pub mod types;
