// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate context;

use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;

#[derive(Debug)]
struct FatStruct {
    pub comment: String,
    pub value: u64,
}

impl Default for FatStruct {
    fn default() -> Self {
        FatStruct{
            comment: String::new(),
            value: 0,
        }
    }
}

// Print the natural numbers from 0 to 9 using a "generator" preserving state on the stack.
fn main() {
    // This method will always `resume()` immediately back to the
    // previous `Context` with a `data` value incremented by one starting at 0.
    // You could thus describe this method as a "natural number generator".
    extern "C" fn context_function(mut t: Transfer<FatStruct>) -> ! {
        for i in 0u64.. {
            print!("Yielding {} => ", i);
            let fat = FatStruct{
                comment: format!("complex value with {}", i),
                value: i,
            };
            t = t.context.resume(Box::new(fat));
        }

        unreachable!();
    }

    // Allocate some stack.
    let stack = ProtectedFixedSizeStack::default();

    // Allocate a Context on the stack.
    let init_fat = FatStruct::default();
    let mut t = Transfer::new(Context::new(&stack, context_function), Box::new(init_fat));

    // Yield 10 times to `context_function()`.
    for _ in 0..10 {
        // Yield to the "frozen" state of `context_function()`.
        // The `data` value is not used in this example and is left at 0.
        // The first and every other call will return references to the actual `Context` data.
        print!("Resuming => ");
        let next_fat = FatStruct::default();
        t = t.context.resume(Box::new(next_fat));

        println!("Got {:?}", t.unpack());
    }

    println!("Finished!");
}
