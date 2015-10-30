//! A simple coroutine implementation, based on underlying context
use context::*;
use stack::Stack;

use std::cell::RefCell;
use std::thread;

pub struct Coroutine(Box<Frame>);

/// Defines return point for `Coroutine::leave`
thread_local!(static G_CONTEXT: RefCell<Option<Context>> = RefCell::new(None) );
/// Service structure which is located on coroutine stack and handles its state
struct Frame {
    /// Jump context
    ///
    /// When coroutine is active, represents return frame
    /// When coroutine is suspended, represents jump-into address
    context: Option<Context>,
    /// Stack handle will be stored on stack itself
    /// This strategy is for future use, when stack will become a trait
    stack:   Stack,
}

impl Coroutine {
    pub fn new<F>(stack: Stack, func: F) -> Coroutine
        where F: FnOnce(isize) -> isize, F: Send + 'static
    {
        use std::ptr;
        use std::mem::transmute;
        unimplemented!();
        
        // TODO: replace with actual code
        let stack_base: *mut () = ptr::null_mut();
        let stack_size = 0usize;
        let func_ptr  : *mut F  = ptr::null_mut();
        
        let frame = Frame {
            context: Some( unsafe {
                Context::new(stack_base, stack_size, thunk::<F>, transmute(func_ptr) )
            } ),
            stack:   stack,
        };

        Coroutine(Box::new(frame));
    }
    /// Enter specified coroutine
    pub fn enter(&mut self, message: isize) -> isize {
        // X is previous frame
        // Y is current frame
        // Z is nested frame
        use std::mem::{replace, swap, transmute};
        let mut frame = &mut self.0.context;
        // 0. Ret = Some(X0), Frame = Some(Z0), Tmp = ???
        // 1. Frame -> Tmp, Frame = None, Tmp = Z0
        let tmp = replace(frame, None).unwrap();
        G_CONTEXT.with(|cell| {
            // We need to deceive borrow checker here
            // Because following jump will stop
            // accessing storage before actual return
            let deceptive_ptr = {
                let mut ret = cell.borrow_mut();
                // 2. Ret <-> Frame, Ret = None, Frame = Some(X0)
                swap(frame, &mut *ret);
                &mut *ret as *mut _
            };
            // 3. jump!
            // Ret = Some(Y0), Frame = Some(X0), Tmp = ???
            // Executing this, we end in leave's POST chunk
            let result = tmp.jump(unsafe { transmute(deceptive_ptr) }, message);
            // POST: we come here after invoking 'leave' on the other side
            // and we need to revert everything
            // frame cell is used to return new frame context
            // 0. Ret = Some(Z1), Frame = Some(X0)

            // Now, we simply swap, and...
            // 1. Ret = Some(X0), Frame = Some(Z1)
            swap(&mut *(cell.borrow_mut()), frame);
            // finally, return
            result
        })
    }
    /// Leave current running context
    pub fn leave(message: isize) -> isize {
        unimplemented!()
    }
    /// Invoked at the end of coroutine to leave it without storing return frame for later use
    fn abandon(message: isize) -> ! {
        unimplemented!()
    }
}

extern fn thunk<F>(message: isize, param: usize) -> !
    where F: FnOnce(isize) -> isize, F: 'static + Send
{
    use std::ptr;
    use std::mem;

    let func = unsafe { ptr::read(mem::transmute::<_, *const F>(param)) };
    let response = {
        func(message)
    };
    Coroutine::abandon(response)
}