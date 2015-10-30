//! A simple coroutine implementation, based on underlying context
use context::*;
use stack::{ Stack, StackSlice };
use std::cell::RefCell;

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
    pub fn new<F>(mut stack: Stack, func: F) -> Coroutine
        where F: FnOnce(isize) -> isize, F: Send + 'static
    {
        use std::ptr;
        use std::mem::transmute;

        let (base, size, frame_ptr, fn_ptr) = {
            let mut slice = StackSlice::new(stack.as_mut_slice());

            let frame_ptr = slice.alloc::<Frame>();
            let fn_ptr    = slice.emplace(func);

            let (base, size) = slice.into_ptr_size();
            (base, size, frame_ptr, fn_ptr)
        };

        unsafe {
            ptr::write(frame_ptr, Frame {
                context: Some( Context::new(base, size, thunk::<F>, transmute(fn_ptr)) ),
                stack:   stack,
            });

            Coroutine(Box::from_raw(frame_ptr))
        }
    }
    /// Enter specified coroutine
    pub fn enter(&mut self, message: isize) -> isize {
        G_CONTEXT.with(|cell| {
            // X is previous frame
            // Y is current frame
            // Z is nested frame
            use std::mem::{replace, swap, transmute};
            let mut frame = &mut self.0.context;
            // 0. Ret = Some(X0), Frame = Some(Z0), Tmp = ???
            // 1. Frame -> Tmp, Frame = None, Tmp = Z0
            let tmp = replace(frame, None).unwrap();
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
        G_CONTEXT.with(|cell| {
            use std::mem::{replace, transmute};
            // Y is previous frame
            // Z is current frame
            // 0. Ret = Some(Y0), Tmp = ???
            let (deceptive_ptr, tmp) = {
                let mut ret = cell.borrow_mut();
                // 1. Ret -> Y0, Ret = None
                let returner = replace(&mut *ret, None).unwrap();
                (&mut *ret as *mut _, returner)
            };
            // 1. Ret = None, Tmp = Y0
            // Jump! then, Ret = Some(Z0), Tmp = ???
            tmp.jump(unsafe { transmute(deceptive_ptr) }, message)
            // POST: we came here after calling 'enter'
            // 0. Ret = Some(Y1), we don't need to do anything explicitly
        })
    }
    // Invoked at the end of coroutine to leave it without storing return frame for later use
    fn abandon(message: isize) -> ! {
        G_CONTEXT.with(|cell| {
            use std::mem::replace;
            // Y is previous frame
            // Z is current frame
            // 0. Ret = Some(Y0), Tmp = ???
            // 1. Ret = None, Tmp = Y0
            let tmp = replace(&mut *(cell.borrow_mut()), None).unwrap();
            // 1. Ret = None, Tmp = Y0
            // Jump into! then, Ret = None, Tmp = ???, and there's no return
            unsafe {
                tmp.jump_into(message)
            }
        });
        unreachable!()
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
