use libc;
use std::ptr;

pub use self::constants::{STACK_ALIGN, STACK_GROWS_DOWN};

type Ptr  = *mut   libc::c_void;
type CPtr = *const libc::c_void;

pub struct Context([u8; constants::CONTEXT_SIZE]);

impl Context {
    pub fn empty() -> Context { Context([0; constants::CONTEXT_SIZE]) }
    pub fn new() -> Context {
        let mut context = Context::empty();
        context
    }

    pub fn save(&mut self) {
        unsafe {
            context_rs_swap(self.0[..].as_mut_ptr() as Ptr, ptr::null());
        }
    }

    pub fn load(&self) -> ! {
        unsafe {
            context_rs_swap(ptr::null_mut(), self.0[..].as_ptr() as CPtr);
        }
        unreachable!("one-way jump")
    }

    pub fn swap(&mut self, other: &Context) {
        unsafe {
            context_rs_swap(self.0[..].as_mut_ptr() as Ptr, other.0[..].as_ptr() as CPtr);
        }
    }
}

#[cfg_attr(
    all(target_arch = "x86_64", target_os = "linux"),
    link(name = "x86_64-linux", kind = "static")
)]
extern {
    fn context_rs_swap(store: Ptr, load: CPtr);
    fn context_rs_init(
        ctxt:       Ptr,
        stack_base: Ptr,
        stack_size: libc::size_t,
        func:       extern fn(libc::intptr_t) -> !,
        data:       libc::intptr_t
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod constants {
    pub const CONTEXT_SIZE:     usize = 0x48; // taken from asm/x86_64-linux.S
    pub const STACK_ALIGN:      usize = 16;
    pub const STACK_GROWS_DOWN: bool  = true;
}
