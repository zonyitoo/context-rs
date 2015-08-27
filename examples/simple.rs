#![feature(libc, rt, fnbox, box_raw)]

extern crate context;
extern crate libc;

use std::rt::util::min_stack;
use std::mem;
use std::boxed::FnBox;

use context::{Context, Stack};

extern "C" fn init_fn(arg: usize, f: *mut libc::c_void) -> ! {
    // Transmute it back to the Box<Box<FnBox()>>
    let func: Box<Box<FnBox()>> = unsafe {
        Box::from_raw(f as *mut Box<FnBox()>)
    };

    // Call it
    func();

    // The argument is the context of the main function
    let ctx: &Context = unsafe { mem::transmute(arg) };
    let mut dummy = Context::empty();
    Context::swap(&mut dummy, ctx);

    unreachable!("Should not reach here!");
}

fn main() {
    // Initialize an empty context
    let mut cur = Context::empty();

    let mut stk = Stack::new(min_stack());
    let ctx = Context::new(init_fn, unsafe { mem::transmute(&cur) }, Box::new(move|| {
        println!("Inside your function!");
    }), &mut stk);

    // Switch!
    Context::swap(&mut cur, &ctx);
}
