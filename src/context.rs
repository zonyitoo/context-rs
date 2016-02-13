// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt;
use std::os::raw::c_void;

use stack::Stack;

// Requires cdecl calling convention on x86, which is the default for "C" blocks.
extern "C" {
    /// Creates a new `Context` ontop of some stack.
    ///
    /// # Arguments
    /// * `sp`   - A pointer to the bottom of the stack.
    /// * `size` - The size of the stack.
    /// * `f`    - A function to be invoked on the first call to jump_fcontext(this, _).
    fn make_fcontext(sp: *mut c_void, size: usize, f: ContextFn) -> &mut Context;

    /// Yields the execution to another `Context`.
    ///
    /// # Arguments
    /// * `to` - A pointer to the `Context` with whom we swap execution.
    /// * `p`  - An arbitrary argument that will be set as the `data` field
    ///          of the `Transfer` object passed to the other Context.
    fn jump_fcontext(to: &Context, p: usize) -> Transfer;

    /// Yields the execution to another `Context` and executes a function "ontop" of it's stack.
    ///
    /// # Arguments
    /// * `to` - A pointer to the `Context` with whom we swap execution.
    /// * `p`  - An arbitrary argument that will be set as the `data` field
    ///          of the `Transfer` object passed to the other Context.
    /// * `f`  - A function to be invoked on `to` before returning.
    fn ontop_fcontext(to: &Context, p: usize, f: ResumeOntopFn) -> Transfer;
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
#[repr(C)]
pub struct Context {
    // NOTE:
    //   - The actual type differs from this one, but that's fine, because `Context` will only
    //     be used using pointers to it's implementation defined struct on it's stack.
    //   - The "placeholder" member has been added to circumvent compiler warnings.
    placeholder: [usize; 0],
}

// NOTE: Rustc is kinda dumb and introduces a overhead of up to 500% compared to the asm methods
//       if we don't explicitely inline them or use LTO (3ns/iter VS. 18ns/iter on i7 3770).
impl Context {
    /// Returns a null-pointer reference to a `Context` struct.
    ///
    /// This method is used in combination with `Transfer::empty()`.
    #[inline(always)]
    pub unsafe fn null_ref() -> &'static Context {
        &*(0 as *const Context)
    }

    /// Allocates a new `Context` ontop of `stack` and returns a reference to it.
    ///
    /// Since the `Context` is allocated on the stack (with an implementation size),
    /// it will be deleted automatically with the `stack`.
    ///
    /// The passed method `f` is not executed until the first call to `resume()`.
    ///
    /// # Warning
    ///
    /// The reference returned by this call is different to the ones returned by `resume()`!
    /// This due to the fact that intially it points to the entry function `f` and only later on
    /// to the actual implementation defined context data on the stack.
    /// Due to this it is not safe to call `resume_ontop()` until after `resume()` has been called.
    #[inline(never)]
    pub fn new<'a>(stack: &'a Stack, f: ContextFn) -> &'a Context {
        unsafe { make_fcontext(stack.top(), stack.len(), f) }
    }

    /// Yields the execution to another `Context`.
    ///
    /// The exact behaviour of this method is implementation defined, but the general mechanism is:
    /// The current state of execution is preserved somewhere and the previously saved state
    /// in the `Context` pointed to by `&self` is restored and executed next.
    ///
    /// This behaviour is similiar in spirit to regular function calls with the difference
    /// that the call to `resume()` only returns when someone resumes the caller in turn.
    #[inline(never)]
    pub fn resume(&self, data: usize) -> Transfer<'static> {
        unsafe { jump_fcontext(self, data) }
    }

    /// Yields the execution to another `Context` and executes a function "ontop" of it's stack.
    ///
    /// This method works similiary to `resume()`.
    /// The difference is that the argument `f` is executed right before the targeted `Context`
    /// pointed to by `&self` is woken up and returns from it's call to `resume()`.
    /// The method `f` is now passed the `Transfer` struct which would normally be returned by
    /// `resume()` and is allowed to inspect and modify it. When `f` is done it
    /// has to return a `Transfer` struct which is then finally the one the `resume()`
    /// method returns in the targeted `Context`.
    ///
    /// This behaviour can be used to either execute additional code or map the `Transfer` struct
    /// to another one before it's returned, without the targeted `Context` giving it's consent.
    /// This behaviour can for instance be used to unwind the stack of an unfinished `Context`,
    /// by calling this method with a function that panics, or to deallocate the own stack,
    /// by deferring the actual deallocation until we are switched back to another, safe `Context`.
    ///
    /// # Warning
    ///
    /// Calling this method is only supported on `Context` references returned
    /// by calls to `resume()`. This is due to the fact that the reference
    /// returned by `new()` points to the entry function instead.
    #[inline(never)]
    pub fn resume_ontop(&self, data: usize, f: ResumeOntopFn) -> Transfer<'static> {
        unsafe { ontop_fcontext(self, data, f) }
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:p}", self)
    }
}

/// This is the return value by `Context::resume()` and `Context::resume_ontop()`.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Transfer<'a> {
    /// The previously executed `Context` which yielded to resume the current one.
    pub context: &'a Context,

    /// The `data` which was passed to `Context::resume()` or
    /// `Context::resume_ontop()` to resume the current `Context`.
    pub data: usize,
}

impl<'a> Transfer<'a> {
    /// Returns a new `Transfer` struct with the members set to their respective arguments.
    #[inline(always)]
    pub fn new(context: &'a mut Context, data: usize) -> Transfer {
        Transfer {
            context: context,
            data: data,
        }
    }

    /// Returns a `Transfer` struct with the `context` member set to a null-pointer reference.
    ///
    /// This method can be used if there is a need to return a `Transfer`
    /// struct but there is no `Context` left to be returned.
    /// This is for instance the case if you use `resume_ontop()` to destroy
    /// the stack as can be seen in the deallocate_ontop() test.
    #[inline(always)]
    pub unsafe fn empty(data: usize) -> Transfer<'static> {
        Transfer {
            context: Context::null_ref(),
            data: data,
        }
    }
}

impl<'a> fmt::Debug for Transfer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Transfer {{ context: {:p}, data: {:p} }}",
               self.context,
               self.data as *const c_void)
    }
}

#[cfg(test)]
mod tests {
    use stack::ProtectedFixedSizeStack;
    use super::*;

    #[test]
    fn new_vs_resume() {
        extern "C" fn noop(mut t: Transfer) -> ! {
            loop {
                t = t.context.resume(0);
            }
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer {
            context: Context::new(&stack, noop),
            data: 0,
        };

        let new_ptr = t.context as *const Context;
        t = t.context.resume(0);
        let resume_ptr = t.context as *const Context;

        assert!(new_ptr != resume_ptr);
    }

    #[test]
    fn number_generator() {
        extern "C" fn number_generator(mut t: Transfer) -> ! {
            for i in 0usize.. {
                t = t.context.resume(i);
            }

            unreachable!();
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer {
            context: Context::new(&stack, number_generator),
            data: 0,
        };

        for i in 0..10usize {
            t = t.context.resume(0);
            assert_eq!(t.data, i);

            if t.data == 9 {
                break;
            }
        }
    }

    #[test]
    fn resume_ontop() {
        extern "C" fn resume(t: Transfer) -> ! {
            t.context.resume_ontop(0, resume_ontop);
            unreachable!();
        }

        extern "C" fn resume_ontop(mut t: Transfer) -> Transfer {
            t.data = 123;
            t
        }

        let stack = ProtectedFixedSizeStack::default();
        let mut t = Transfer {
            context: Context::new(&stack, resume),
            data: 0,
        };

        t = t.context.resume(0);
        assert_eq!(t.data, 123);
    }
}
