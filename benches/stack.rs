// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![feature(test)]

extern crate context;
extern crate test;

use context::stack::{Stack, FixedSizeStack};
use test::Bencher;

#[bench]
fn stack_alloc(b: &mut Bencher) {
    b.iter(|| FixedSizeStack::default());
}

#[bench]
fn regular_alloc(b: &mut Bencher) {
    b.iter(|| Vec::<u8>::with_capacity(Stack::default_size()));
}
