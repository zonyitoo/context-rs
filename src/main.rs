#![feature(asm)]
#![feature(rustc_private)]
#![feature(box_raw)]
#![feature(fnbox)]
#![feature(rt)]
#[macro_use]
extern crate log;
extern crate context;
extern crate libc; 

use context::Context;
use context::Stack;

use std::mem::transmute;
use std::sync::mpsc::channel;                                          
use std::rt::util::min_stack;
use std::rt::unwind::try;
use std::boxed::FnBox;

extern "C" fn init_fn(arg: usize, f: *mut libc::c_void) {         
    let func: Box<Box<FnBox()>> = unsafe {
        Box::from_raw(f as *mut Box<FnBox()>)
    };
    if let Err(cause) = unsafe { try(move|| func()) } {
        error!("Panicked inside: {:?}", cause.downcast::<&str>());
    }

    let ctx: &Context = unsafe { transmute(arg) };
    Context::load(ctx);
}


fn main() {
    let mut cur = Context::empty();
    let (tx, rx) = channel();

    let mut stk = Stack::new(min_stack());
    let ctx = Context::new(init_fn, unsafe { transmute(&cur) }, Box::new(move|| {
        tx.send(1).unwrap();
    }), &mut stk);

    assert!(rx.try_recv().is_err());

    let mut _no_use = Box::new(true);
    Context::save(&mut cur);
    if *_no_use {
        *_no_use = false;
        Context::load(&ctx);
        // would never come here!!
        return;
    }

    assert_eq!(rx.recv().unwrap(), 1);
}
