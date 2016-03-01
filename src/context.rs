// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::os::raw::c_void;
use std::fmt;

use stack::Stack;

// Requires cdecl calling convention on x86, which is the default for "C" blocks.
extern "C" {
    /// Creates a new `Context` ontop of some stack.
    ///
    /// # Arguments
    /// * `sp`   - A pointer to the bottom of the stack.
    /// * `size` - The size of the stack.
    /// * `f`    - A function to be invoked on the first call to jump_fcontext(this, _).
    #[inline(never)]
    fn make_fcontext(sp: *mut c_void, size: usize, f: ContextFn) -> &'static c_void;

    /// Yields the execution to another `Context`.
    ///
    /// # Arguments
    /// * `to` - A pointer to the `Context` with whom we swap execution.
    /// * `p`  - An arbitrary argument that will be set as the `data` field
    ///          of the `Transfer` object passed to the other Context.
    #[inline(never)]
    fn jump_fcontext(to: &'static c_void, p: usize) -> Transfer;

    /// Yields the execution to another `Context` and executes a function "ontop" of it's stack.
    ///
    /// # Arguments
    /// * `to` - A pointer to the `Context` with whom we swap execution.
    /// * `p`  - An arbitrary argument that will be set as the `data` field
    ///          of the `Transfer` object passed to the other Context.
    /// * `f`  - A function to be invoked on `to` before returning.
    #[inline(never)]
    fn ontop_fcontext(to: &'static c_void, p: usize, f: ResumeOntopFn) -> Transfer;
}

/// Functions of this signature are used as the entry point for a new `Context`.
pub type ContextFn = extern "C" fn(t: Transfer) -> !;

/// Functions of this signature are used as the callback after resuming ontop of a `Context`.
pub type ResumeOntopFn = extern "C" fn(t: Transfer) -> Transfer;

/// A `Context` provides the capability of saving and restoring the current state of execution.
///
/// If we have 2 or more `Context` instances, we can thus easily "freeze" the
/// current state of execution and explicitely switch to another `Context`.
/// This `Context` is then resumed exactly where it left of and
/// can in turn "freeze" and switch to another `Context`.
///
/// # Examples
///
/// See [examples/basic.rs](https://github.com/zonyitoo/context-rs/blob/master/examples/basic.rs)
// The reference is using 'static because we can't possibly imply the
// lifetime of the Context instances returned by resume() anyways.
#[repr(C)]
pub struct Context(&'static c_void);

// NOTE: Rustc is kinda dumb and introduces a overhead of up to 500% compared to the asm methods
//       if we don't explicitely inline them or use LTO (e.g.: 3ns/iter VS. 18ns/iter on i7 3770).
impl Context {
    /// Creates a new `Context` prepared to execute `f` at the beginning of `stack`.
    ///
    /// `f` is not executed until the first call to `resume()`.
    #[inline(always)]
    pub fn new(stack: &Stack, f: ContextFn) -> Context {
        Context(unsafe { make_fcontext(stack.top(), stack.len(), f) })
    }

    /// Yields the execution to another `Context`.
    ///
    /// The exact behaviour of this method is implementation defined, but the general mechanism is:
    /// The current state of execution is preserved somewhere and the previously saved state
    /// in the `Context` pointed to by `self` is restored and executed next.
    ///
    /// This behaviour is similiar in spirit to regular function calls with the difference
    /// that the call to `resume()` only returns when someone resumes the caller in turn.
    ///
    /// The returned `Transfer` struct contains the previously active `Context` and
    /// the `data` argument used to resume the current one.
    #[inline(always)]
    pub fn resume(self, data: usize) -> Transfer {
        unsafe { jump_fcontext(self.0, data) }
    }

    /// Yields the execution to another `Context` and executes a function "ontop" of it's stack.
    ///
    /// This method identical to `resume()` with a minor difference:
    ///
    /// The argument `f` is executed right before the targeted `Context` pointed to by `self`
    /// is woken up and returns from it's call to `resume()`. The method `f` is now passed the
    /// `Transfer` struct which would normally be returned by `resume()` and is allowed to inspect
    /// and modify it. When `f` is done it has to return a `Transfer` struct which is then finally
    /// the one the `resume()` method returns in the targeted `Context`.
    ///
    /// This behaviour can be used to either execute additional code or map the `Transfer` struct
    /// to another one before it's returned, without the targeted `Context` giving it's consent.
    /// For instance it can be used to unwind the stack of an unfinished `Context`,
    /// by calling this method with a function that panics, or to deallocate the own stack,
    /// by deferring the actual deallocation until we jumped to another, safe `Context`.
    #[inline(always)]
    pub fn resume_ontop(self, data: usize, f: ResumeOntopFn) -> Transfer {
        unsafe { ontop_fcontext(self.0, data, f) }
    }
}

/// This is the return value by `Context::resume()` and `Context::resume_ontop()`.
impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Context({:p})", self.0)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Transfer {
    /// The previously executed `Context` which yielded to resume the current one.
    pub context: Context,

    /// The `data` which was passed to `Context::resume()` or
    /// `Context::resume_ontop()` to resume the current `Context`.
    pub data: usize,
}

impl Transfer {
    /// Returns a new `Transfer` struct with the members set to their respective arguments.
    #[inline(always)]
    pub fn new(context: Context, data: usize) -> Transfer {
        Transfer {
            context: context,
            data: data,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::os::raw::c_void;

    use stack::ProtectedFixedSizeStack;
    use super::*;

    #[test]
    fn type_sizes() {
        assert_eq!(mem::size_of::<Context>(), mem::size_of::<usize>());
        assert_eq!(mem::size_of::<Context>(), mem::size_of::<*const c_void>());
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn stack_alignment() {
        #[allow(non_camel_case_types)]
        #[repr(simd)]
        struct u8x16(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8);

        extern "C" fn context_function(t: Transfer) -> ! {
            // If we do not use an array in combination with mem::uninitialized(),
            // Rust will still generate SSE/NEON operations for the assignment
            // and make the test crash with a segmentation fault due to misalignment.
            let data: [u8x16; 1] = unsafe { mem::uninitialized() };
            let addr = &data as *const _ as usize;

            t.context.resume(addr % mem::align_of::<u8x16>());
            unreachable!();
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer::new(Context::new(&stack, context_function), 0);

        t = t.context.resume(0);
        assert_eq!(t.data, 0);
    }

    #[test]
    fn number_generator() {
        extern "C" fn context_function(mut t: Transfer) -> ! {
            for i in 0usize.. {
                assert_eq!(t.data, i);
                t = t.context.resume(i);
            }

            unreachable!();
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer::new(Context::new(&stack, context_function), 0);

        for i in 0..10usize {
            t = t.context.resume(i);
            assert_eq!(t.data, i);

            if t.data == 9 {
                break;
            }
        }
    }

    #[test]
    fn resume_ontop() {
        extern "C" fn resume(t: Transfer) -> ! {
            assert_eq!(t.data, 0);
            t.context.resume_ontop(1, resume_ontop);
            unreachable!();
        }

        extern "C" fn resume_ontop(mut t: Transfer) -> Transfer {
            assert_eq!(t.data, 1);
            t.data = 123;
            t
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer::new(Context::new(&stack, resume), 0);

        t = t.context.resume(0);
        assert_eq!(t.data, 123);
    }
}
