// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![feature(test)]

extern crate context;
extern crate test;

use context::stack::{Stack, FixedSizeStack, ProtectedFixedSizeStack};
use test::Bencher;

#[bench]
fn stack_alloc_reference_perf(b: &mut Bencher) {
    b.iter(|| test::black_box(Vec::<u8>::with_capacity(Stack::default_size())));
}

#[bench]
fn stack_alloc_fixed(b: &mut Bencher) {
    b.iter(|| test::black_box(FixedSizeStack::default()));
}

#[bench]
fn stack_alloc_protected_fixed(b: &mut Bencher) {
    b.iter(|| test::black_box(ProtectedFixedSizeStack::default()));
}
