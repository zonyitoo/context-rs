//! A simple coroutine implementation, based on underlying context
use context::Context;
use stack::Stack;
use std::cell::RefCell;

pub struct Coroutine<T: Stack> {
    context: Option<Context>,
    stack:   T,
}

/// Defines return point for `Coroutine::leave`
thread_local!(static G_CONTEXT: RefCell<Option<Context>> = RefCell::new(None) );

impl<T: Stack> Coroutine<T> {
    pub fn new<F>(mut stack: T, func: F) -> Coroutine<T>
        where F: FnOnce(isize) -> isize, F: Send + 'static
    {
        use std::mem::transmute;

        let mut slice = to_stack_slice(stack.as_slice());
        let fn_ptr    = emplace(&mut slice, func);

        Coroutine {
            context: Some( unsafe {
                Context::new(
                    slice.0 as *mut (),
                    slice.1,
                    thunk::<F>,
                    transmute(fn_ptr)
                )
            } ),
            stack:   stack,
        }
    }
    /// Deconstruct coroutine and return back its stack
    ///
    /// This is safe only in case coroutine is finished
    /// Otherwise, it will cause resource leaks,
    /// since no resources inside coroutine will be properly freed
    pub fn detach_stack(self) -> T {
        self.stack
    }
    /// Check if coroutine has already finished
    ///
    /// You can't do anything useful with finished coroutine,
    /// except detach its stack for later reuse
    pub fn is_finished(&self) -> bool {
        self.context.is_none()
    }
    /// Enter specified coroutine
    pub fn enter(&mut self, message: isize) -> isize {
        G_CONTEXT.with(|cell| {
            // X is previous frame
            // Y is current frame
            // Z is nested frame
            use std::mem::{replace, swap, transmute};
            // 0. Ret = Some(X0), Frame = Some(Z0), Tmp = ???
            // 1. Frame -> Tmp, Frame = None, Tmp = Z0
            let tmp = replace(&mut self.context, None).unwrap();
            // We need to deceive borrow checker here
            // Because following jump will stop
            // accessing storage before actual return
            let deceptive_ptr = {
                let mut ret = cell.borrow_mut();
                // 2. Ret <-> Frame, Ret = None, Frame = Some(X0)
                swap(&mut self.context, &mut *ret);
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
            swap(&mut *(cell.borrow_mut()), &mut self.context);
            // finally, return
            result
        })
    }
}
/// Leave current running coroutine
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
/// Invoked at the end of coroutine to leave it without storing return frame for later use
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

extern fn thunk<F>(message: isize, param: usize) -> !
    where F: FnOnce(isize) -> isize, F: 'static + Send
{
    use std::ptr;
    use std::mem;

    let func = unsafe { ptr::read(mem::transmute::<_, *const F>(param)) };
    let response = {
        func(message)
    };
    abandon(response)
}

type StackSlice = (*mut u8, usize);

fn to_stack_slice(slice: &mut [u8]) -> StackSlice {
    let base = to_base_ptr(slice);
    return (base, slice.len());

    #[cfg(not(stack_grows_up))]
    fn to_base_ptr(slice: &mut [u8]) -> *mut u8
    {
        let len = slice.len();
        unsafe {
            slice.as_mut_ptr().offset(len as isize)
        }
    }

    #[cfg(stack_grows_up)]
    fn to_base_ptr(slice: &mut [u8]) -> *mut u8
    {
        slice.as_mut_ptr()
    }
}

fn emplace<T>(slice: &mut StackSlice, value: T) -> *mut T {
    use std::ptr;

    let ptr = alloc_val(slice, &value);
    unsafe {
        ptr::write(ptr, value);
    }
    ptr
}

fn alloc_val<T>(slice: &mut StackSlice, _val: &T) -> *mut T {
    alloc(slice)
}

fn alloc<T>(slice: &mut StackSlice) -> *mut T {
    use std::mem;
    // we'll need these to place T properly
    let size  = mem::size_of::<T>();
    let align = mem::align_of::<T>();

    return aligned(slice, size, align) as *mut T;

    // advances stack base with raw offset
    fn advance_raw(slice: &mut StackSlice, bytes: isize) {
        let size = bytes as usize;
        assert!(slice.1 >= size);
        unsafe { slice.0 = slice.0.offset(bytes); }
        slice.1 -= size;
    }

    #[cfg(not(stack_grows_up))]
    fn aligned(slice: &mut StackSlice, size: usize, align: usize) -> *mut () {
        use std::mem::transmute;
        // 1. Allocate enough
        advance_raw(slice, -(size as isize));
        // 2a. Compute align diff, down
        let pt: usize = unsafe { transmute(slice.0) };
        let delta = pt % align;
        // 2b. Align stack to this boundary
        advance_raw(slice, -(delta as isize));
        slice.0 as *mut ()
    }
    #[cfg(stack_grows_up)]
    fn aligned(slice: &mut StackSlice, size: usize, align: usize) -> *mut () {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn single_enter_abandon() {
        use stack::ScopedStack;
        use coroutine::Coroutine;
        let mut buffer = [0u8; 8 * 1024];

        let mut coro = Coroutine::new(
            ScopedStack::new(&mut buffer),
            |_m| {
                _m + 42
            }
        );

        let r1 = coro.enter(11);
        
        assert_eq!(r1, 53);
        assert!(coro.is_finished());
    }
}