// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![cfg_attr(feature = "nightly", feature(panic_propagate, recover, std_panic))]

extern crate context;

#[cfg(not(feature = "nightly"))]
mod imp {
    pub fn run() {
        println!("Rust Nightly is required for this example!");
    }
}

#[cfg(feature = "nightly")]
mod imp {
    use std::panic;

    use context::{Context, Transfer};
    use context::stack::ProtectedFixedSizeStack;

    // This struct is used to demonstrate that the stack is actually being unwound.
    struct Dropper;

    impl Drop for Dropper {
        fn drop(&mut self) {
            println!("Dropping a Dropper!");
        }
    }

    fn take_some_stack_from_transfer(t: Transfer) -> Option<ProtectedFixedSizeStack> {
        let stack_ref = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
        stack_ref.take()
    }

    fn stack_ref_from_some_stack(some_stack: &mut Option<ProtectedFixedSizeStack>) -> usize {
        some_stack as *mut Option<ProtectedFixedSizeStack> as usize
    }

    // This method is used to force unwind a foreign context function.
    extern "C" fn unwind_stack(t: Transfer) -> Transfer {
        println!("Unwinding stack by panicking!");

        // We need to store the t.context to the `t` inside `context_function`, because this function
        // won't be able to pass the correct `t.context` to `context_function`, which is the correct
        // return Context after stack unwinding.
        let t_inside_context_function = unsafe {
            &mut *(t.data as *mut Transfer)
        };
        t_inside_context_function.context = t.context;

        // Unwind the current stack by panicking.
        // We use std::panic::propagate() here however because panic!() will call the panic handler
        // which aborts the process if more than one panic is triggered on a thread.
        // This is problematic for coroutines though (the most popular use case for this crate),
        // because by their very definition multiple seperate stacks per thread are used.
        //
        // Thus the following problem can occur:
        //   Let's say we have a `Context` function (a coroutine) which _owns_ a list of coroutines.
        //   If that primary coroutine unwinds it's stack by panicking it will drop the list and thus
        //   all the contained coroutines are dropped. When they unwind their stack due to this the
        //   process will be aborted by Rust's runtime with "thread panicked while it is panicking".
        //
        // The downside of this technique is however that the internaL PANIC_COUNT is off by one
        // (it's still zero) and thus it won't abort the process anymore if drop() panics.
        // This could be fixed however by setting a custom panic handler using panic::set_handler().
        struct ForceUnwind;
        panic::propagate(Box::new(ForceUnwind));
    }

    // This method is used to defer stack deallocation after it's not used anymore.
    extern "C" fn delete_stack(t: Transfer) -> Transfer {
        println!("Deleting stack!");
        let _ = take_some_stack_from_transfer(t);

        t
    }

    // This method is used as the "main" context function.
    extern "C" fn context_function(mut t: Transfer) -> ! {
        println!("Entering context_function...");

        // Take over the stack from the main function, because we want to manage it ourselves.
        // The main function could safely return after this in theory.
        let mut some_stack = take_some_stack_from_transfer(t);
        let stack_ref = stack_ref_from_some_stack(&mut some_stack);

        let t_ptr = &mut t as *mut _ as usize;

        let result = {
            // Use `std::panic::recover()` to catch panics from `unwind_stack()`.
            panic::recover(|| {
                // We use an instance of `Dropper` to demonstrate
                // that the stack is actually being unwound.
                let _dropper = Dropper;

                let mut t = unsafe { &mut *(t_ptr as *mut Transfer) };

                // We've set everything up! Go back to `main()`!
                println!("Everything's set up!");
                *t = t.context.resume(t_ptr);

                for i in 0usize.. {
                    print!("Yielding {} => ", i);
                    *t = t.context.resume(i);
                }
            })
        };

        match result {
            Ok(..) => println!("Finished loop without panicking (this should not happen here)!"),
            Err(..) => println!("Recovered from a panic!"),
        }

        // We own the stack (`main()` gave it to us) and we need to delete it.
        // Since it would be unsafe to do so while we're still in the context function running on
        // that particular stack, we defer deletion of it by resuming `main()` and running the ontop
        // function `delete_stack()` before `main()` returns from it's call to `resume_ontop()`.
        println!("Defer stack deallocation by returning to main()!");
        t.context.resume_ontop(stack_ref, delete_stack);

        unreachable!();
    }

    pub fn run() {
        // Allocate some stack.
        let mut some_stack = Some(ProtectedFixedSizeStack::default());
        let stack_ref = stack_ref_from_some_stack(&mut some_stack);

        // Allocate a Context on the stack.
        // `t` will now contain a reference to the context function
        // `context_function()` and a `data` value of 0.
        let mut t = Transfer {
            context: Context::new(some_stack.as_ref().unwrap(), context_function),
            data: 0,
        };

        // Yield to context_function(). This important since the returned `Context` reference is
        // different than the one returned by `Context::new()` (since it points to the entry function).
        // It's important that we do this first or else calling `Context::resume_ontop()` will crash.
        // See documentation of `Context::resume_ontop()` for more information.
        // Furthermore we pass a reference to the Option<ProtectedFixedSizeStack> along with it
        // so it can delete it's own stack (which is important for stackful coroutines).
        t = t.context.resume(stack_ref);

        // Store the pointer to the `t` inside the context_function for `unwind_stack`.
        let t_ptr_inside_context_function = t.data;

        // Yield 10 times to `context_function()`.
        for _ in 0..10 {
            // Yield to the "frozen" state of `context_function()`.
            // The `data` value is not used in this example and is left at 0.
            print!("Resuming => ");
            t = t.context.resume(0);

            // `t` will now contain a reference to the `Context` which `resumed()` us
            // (here: `context_function()`) and the value passed to it.
            println!("Got {}", t.data);
        }

        // Resume `context_function()` with the ontop function `unwind_stack()`.
        // Before it returns from it's own call to `resume()` it will call `unwind_stack()`.
        println!("Resuming context with unwind_stack() ontop!");
        t.context.resume_ontop(t_ptr_inside_context_function, unwind_stack);

        match some_stack {
            Some(..) => println!("Stack is still there (this should not happen here)!"),
            None => println!("Stack has been deleted!"),
        }

        println!("Finished!");
    }
}

fn main() {
    imp::run();
}
