#![allow(unused_features)]
#![feature(asm, rustc_private, rt, fnbox, box_raw)]
#![feature(core_simd)]

#[macro_use]
extern crate log;
extern crate libc;
extern crate memmap;

pub use context::Context;
pub use stack::Stack;

pub mod context;
pub mod stack;
mod sys;
