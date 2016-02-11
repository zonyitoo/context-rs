// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate context;

use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;

// Print the natural numbers from 0 to 9 using a "generator" preserving state on the stack.
fn main() {
    // This method will always `resume()` immediately back to the
    // previous `Context` with a `data` value of the next number in the fibonacci sequence.
    // You could thus describe this method as a "fibonacci sequence generator".
    extern "C" fn context_function(t: Transfer) {
        let mut a = 0usize;
        let mut b = 1usize;

        loop {
            print!("Yielding {} => ", a);
            t.context.resume(a);

            let next = a + b;
            a = b;
            b = next;
        }
    }

    // Allocate some stack.
    let stack = ProtectedFixedSizeStack::default();

    // Allocate a Context on the stack.
    // `t` will now contain a reference to the context function
    // `context_function()` and a `data` value of 0.
    let mut t = Transfer {
        context: Context::new(&stack, context_function),
        data: 0,
    };

    // Yield 10 times to `context_function()`.
    for _ in 0..10 {
        // Yield to the "frozen" state of `context_function()`.
        // The `data` value is not used in this example and is left at 0.
        // The first and every other call will return references to the actual `Context` data.
        print!("Resuming => ");
        t = t.context.resume(0);

        // `t` will now contain a reference to the `Context` which `resumed()` us
        // (here: `context_function()`) and the value passed to it.
        println!("Got {}", t.data);
    }

    println!("Finished!");
}
