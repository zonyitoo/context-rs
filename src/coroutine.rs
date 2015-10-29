//! A simple coroutine implementation, based on underlying context
use context::*;
use stack::Stack;

pub struct Coroutine {
    frame: Box<Frame>
}
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
        unimplemented!()
    }
    /// Enter specified coroutine
    pub fn enter(&mut self, message: isize) -> isize {
        unimplemented!()
    }
    /// Leave current running context
    pub fn leave(message: isize) -> isize {
        unimplemented!()
    }
}

extern fn thunk<F>(message: isize, param: usize) -> !
    where F: FnOnce(isize) -> isize, F: 'static + Send
{
    use std::ptr;
    use std::mem;

    let func = unsafe { ptr::read(mem::transmute::<_, *const F>(param)) };
    let responce = func(message);
    unreachable!()
}