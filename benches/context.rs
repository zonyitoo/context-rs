// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![feature(test)]

extern crate context;
extern crate test;

use test::Bencher;

use context::{Context, Transfer};
use context::stack::FixedSizeStack;
use std::mem;

#[bench]
fn resume_reference_perf(b: &mut Bencher) {
    #[inline(never)]
    extern "C" fn yielder(t: Transfer) -> Transfer {
        test::black_box(t)
    }

    b.iter(|| yielder(unsafe { Transfer::empty(0) }));
}

#[bench]
fn resume(b: &mut Bencher) {
    extern "C" fn yielder(mut t: Transfer) -> ! {
        loop {
            t = t.context.resume(1);
        }
    }

    let stack = FixedSizeStack::default();
    let mut t = Transfer::new(Context::new(&stack, yielder), 0);

    b.iter(|| unsafe { t = mem::replace(&mut t, mem::uninitialized()).context.resume(0) });
}
