//! Relatively safe wrapper around context changing functions
#![allow(improper_ctypes)]

use std::ptr;
use std::mem;

/// Opaque wrapper around one-time context switch handle
///
/// Jump point is one-time and non-cloneable.
/// This ensures that any specific coroutine constructed with it
/// can be reached only once
pub struct Context(*const ());

impl Context {
    /// Constructs new initialized context handle
    ///
    /// # Unsafe
    /// A user must ensure proper stack base pointer and stack size
    pub unsafe fn new(
        stack_base: *mut (),
        stack_size: usize,
        func      : extern fn(isize, usize) -> !,
        param     : usize
    ) -> Context {
        Context(make_fcontext(stack_base, stack_size, func, param))
    }
    /// Switches to provided context, storing current state in process
    ///
    /// Since context handle is one-time, it's consumed in process
    pub fn jump(self, store: &mut Option<Context>, message: isize) -> isize {
        // Store must be None - new context will be stored inside
        // Wanna reuse existing - clean it manually
        assert!(
            store.is_none(),
            "expected empty storage"
        );
        *store = Some(Context(ptr::null()));
        self.jump_impl(
            &mut (store.as_mut().unwrap().0) as *mut _,
            message
        )
    }
    /// Switches to context, without storing current one
    ///
    /// # Unsafe
    /// Should be used only at the end of coroutine, otherwise
    /// will result in resource leak or other nasty consequences
    pub unsafe fn jump_into(self, message: isize) -> ! {
        self.jump_impl(ptr::null_mut(), message);
        unreachable!("one-way jump")
    }

    fn jump_impl(self, store: *mut *const (), message: isize) -> isize {
        let dest = self.0;
        mem::forget(self);
        unsafe {
            jump_fcontext(store, dest, message)
        }
    }
}
/// Helper function for jumping between nullable Context handles
pub fn jump(cell: &mut Option<Context>, store: &mut Option<Context>, message: isize) -> isize {
    mem::replace(cell, None).unwrap().jump(store, message)
}
/// Helper function for jumping out of nullable Context
pub unsafe fn jump_into(cell: &mut Option<Context>, message: isize) -> ! {
    mem::replace(cell, None).unwrap().jump_into(message)
}

extern {
    /// Switches from current execution context to specified one
    /// Stores current state on top of current stack 
    fn jump_fcontext(
        old_ctxt: *mut *const (),
        new_ctxt:      *const (),
        message:       isize
    ) -> isize;
    // 1. Put reversed dummy state on top of provided stack
    // 2. Change dummy's IP so that it will start with provided func
    // 3. Return newly acquired context pointer
    fn make_fcontext(
        stack_base: *mut (),
        stack_size: usize,
        func:       extern fn(isize, usize) -> !,
        data:       usize // will be passed to func as second argument
    ) -> *const ();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[cfg(not(stack_grows_up))]
    fn base(slice: &mut[u8]) -> (*mut (), usize) {
        unsafe {
            let len = slice.len();
            (slice.as_mut_ptr().offset(len as isize) as *mut (), len)
        }
    }

    #[cfg(stack_grows_up)]
    fn base(slice: &mut[u8]) -> (*mut (), usize) {
        unsafe {
            (slice.as_mut_ptr() as *mut (), slice.len())
        }
    }

    static mut g_contexts:  (Option<Context>, Option<Context>) = (None, None);

    const PARAM: usize = 123456usize;
    const TOP0 : isize = 0;
    const TOP1 : isize = -18; 

    const CORO0: isize = 42;
    const CORO1: isize = -99;

    /*
        Test success markers
    */
    static mut param_ok: bool = false;
    static mut top0_ok : bool = false;
    static mut top1_ok : bool = false;

    extern "C" fn raw_coroutine(msg: isize, param: usize) -> ! {
        unsafe {
            let contexts = &mut g_contexts;
            param_ok = param == PARAM;
            top0_ok  = msg   == TOP0;

            let msg  = jump(&mut contexts.0, &mut contexts.1, CORO0);

            top1_ok  = msg == TOP1;

            jump_into(&mut contexts.0, CORO1)
        }
    }

    #[test]
    fn raw_api() {
        unsafe {
            // Allocate local slice stack, 8KiB should be enough for most cases
            let mut memchunk = [0u8; 8192];
            let (base, size) = base(&mut memchunk[..]);
            let contexts = &mut g_contexts;

            contexts.1 = Some(Context::new(base, size, raw_coroutine, PARAM));
            
            let coro = jump(&mut contexts.1, &mut contexts.0, TOP0);

            assert!(param_ok);
            assert!(top0_ok);
            assert!(coro == CORO0);

            let coro = jump(&mut contexts.1, &mut contexts.0, TOP1);

            assert!(top1_ok);
            assert!(coro == CORO1);
        }
    }
}
