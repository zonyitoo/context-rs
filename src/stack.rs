// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ptr;
use std::sync::atomic;
use std::env;
use std::fmt;

use libc;

use memmap::{Mmap, MmapOptions, Protection};

/// A task's stack. The name "Stack" is a vestige of segmented stacks.
pub struct Stack {
    buf: Option<Mmap>,
    min_size: usize,
}

impl fmt::Debug for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "Stack {} buf: ", "{"));
        match self.buf {
            Some(ref map) => try!(write!(f, "Some({:#x}), ", map.ptr() as libc::uintptr_t)),
            None => try!(write!(f, "None, ")),
        }
        write!(f, "min_size: {:?} {}", self.min_size, "}")
    }
}

impl Stack {
    /// Allocate a new stack of `size`. If size = 0, this will fail. Use
    /// `dummy_stack` if you want a zero-sized stack.
    pub fn new(size: usize) -> Stack {
        // Map in a stack. Eventually we might be able to handle stack
        // allocation failure, which would fail to spawn the task. But there's
        // not many sensible things to do on OOM.  Failure seems fine (and is
        // what the old stack allocation did).
        let options = MmapOptions { stack: true };
        let stack = match Mmap::anonymous_with_options(size, Protection::ReadCopy, options) {
            Ok(map) => map,
            Err(e) => panic!("mmap for stack of size {} failed: {}", size, e)
        };

        // Change the last page to be inaccessible. This is to provide safety;
        // when an FFI function overflows it will (hopefully) hit this guard
        // page. It isn't guaranteed, but that's why FFI is unsafe. buf.data is
        // guaranteed to be aligned properly.
        if !protect_last_page(&stack) {
            panic!("Could not memory-protect guard page. stack={:?}",
                  stack.ptr());
        }

        Stack {
            buf: Some(stack),
            min_size: size,
        }
    }

    /// Create a 0-length stack which starts (and ends) at 0.
    #[allow(dead_code)]
    pub unsafe fn dummy_stack() -> Stack {
        Stack {
            buf: None,
            min_size: 0,
        }
    }

    #[allow(dead_code)]
    pub fn guard(&self) -> *const usize {
        (self.start() as usize + page_size()) as *const usize
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { self.buf.as_mut().unwrap().as_mut_slice() }
    }

    /// Point to the low end of the allocated stack
    pub fn start(&self) -> *const usize {
        self.buf.as_ref()
            .map(|m| m.ptr() as *const usize)
            .unwrap_or(ptr::null())
    }

    /// Point one usize beyond the high end of the allocated stack
    pub fn end(&self) -> *const usize {
        self.buf
            .as_ref()
            .map(|buf| unsafe {
                buf.ptr().offset(buf.len() as isize) as *const usize
            })
            .unwrap_or(ptr::null())
    }
}

#[cfg(unix)]
fn protect_last_page(stack: &Mmap) -> bool {
    unsafe {
        // This may seem backwards: the start of the segment is the last page?
        // Yes! The stack grows from higher addresses (the end of the allocated
        // block) to lower addresses (the start of the allocated block).
        let last_page = stack.ptr() as *mut libc::c_void;
        libc::mprotect(last_page, page_size() as libc::size_t,
                       libc::PROT_NONE) != -1
    }
}

#[cfg(windows)]
fn protect_last_page(stack: &Mmap) -> bool {
    unsafe {
        // see above
        let last_page = stack.ptr() as *mut libc::c_void;
        let mut old_prot: libc::DWORD = 0;
        libc::VirtualProtect(last_page, page_size() as libc::SIZE_T,
                             libc::PAGE_NOACCESS,
                             &mut old_prot as libc::LPDWORD) != 0
    }
}

#[derive(Debug)]
pub struct StackPool {
    // Ideally this would be some data structure that preserved ordering on
    // Stack.min_size.
    stacks: Vec<Stack>,
}

impl StackPool {
    pub fn new() -> StackPool {
        StackPool {
            stacks: vec![],
        }
    }

    pub fn take_stack(&mut self, min_size: usize) -> Stack {
        // Ideally this would be a binary search
        match self.stacks.iter().position(|s| min_size <= s.min_size) {
            Some(idx) => self.stacks.swap_remove(idx),
            None => Stack::new(min_size)
        }
    }

    pub fn give_stack(&mut self, stack: Stack) {
        if self.stacks.len() <= max_cached_stacks() {
            self.stacks.push(stack)
        }
    }
}

fn max_cached_stacks() -> usize {
    static mut AMT: atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;
    match unsafe { AMT.load(atomic::Ordering::SeqCst) } {
        0 => {}
        n => return n - 1,
    }
    let amt = env::var("RUST_MAX_CACHED_STACKS").ok().and_then(|s| s.parse().ok());
    // This default corresponds to 20M of cache per scheduler (at the
    // default size).
    let amt = amt.unwrap_or(10);
    // 0 is our sentinel value, so ensure that we'll never see 0 after
    // initialization has run
    unsafe { AMT.store(amt + 1, atomic::Ordering::SeqCst); }
    return amt;
}

#[cfg(unix)]
fn page_size() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_PAGESIZE) as usize
    }
}

#[cfg(windows)]
fn page_size() -> usize {
    use std::mem;

    unsafe {
        let mut info = mem::zeroed();
        libc::GetSystemInfo(&mut info);
        info.dwPageSize as usize
    }
}

#[cfg(test)]
mod tests {
    use super::StackPool;

    #[test]
    fn stack_pool_caches() {
        let mut p = StackPool::new();
        let s = p.take_stack(10);
        p.give_stack(s);
        let s = p.take_stack(4);
        assert_eq!(s.min_size, 10);
        p.give_stack(s);
        let s = p.take_stack(14);
        assert_eq!(s.min_size, 14);
        p.give_stack(s);
    }

    #[test]
    fn stack_pool_caches_exact() {
        let mut p = StackPool::new();
        let s = p.take_stack(10);
        p.give_stack(s);

        let s = p.take_stack(10);
        assert_eq!(s.min_size, 10);
    }
}
