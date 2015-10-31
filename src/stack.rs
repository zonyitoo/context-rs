// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use memmap::Mmap;

/// Just in case someone would want something different than normal FixedSizeStack
pub trait Stack {
    fn as_slice(&mut self) -> &mut [u8];
}

#[cfg(unix)]
pub fn page_size() -> usize {
    use libc::{sysconf, _SC_PAGESIZE};
    unsafe {
        sysconf(_SC_PAGESIZE) as usize
    }
}

#[cfg(windows)]
pub fn page_size() -> usize {
    use std::mem;
    use libc::GetSystemInfo;

    unsafe {
        let mut info = mem::zeroed();
        GetSystemInfo(&mut info);
        info.dwPageSize as usize
    }
}
/// Fixed-size stack, with guard page at the end
pub struct FixedSizeStack(Mmap);

impl FixedSizeStack {
    /// Allocate a new stack of `size`. If size = 0, this will fail
    pub fn new(size: usize) -> FixedSizeStack {
        use memmap::{MmapOptions, Protection};
        FixedSizeStack(
            Mmap::anonymous_with_options(
                size,
                Protection::ReadCopy,
                MmapOptions { stack: true }
            ).unwrap()
        )
    }
}

impl Stack for FixedSizeStack {
    fn as_slice(&mut self) -> &mut [u8] {
        unsafe { self.0.as_mut_slice() }
    }
}

pub struct ProtectedStack(FixedSizeStack);

impl ProtectedStack {
    pub fn new(size: usize) -> ProtectedStack {
        let mut buf = FixedSizeStack::new(size + page_size());
        protect_page(split_last_page(buf.as_slice()).0);
        ProtectedStack(buf)
    }
}

impl Stack for ProtectedStack {
    fn as_slice(&mut self) -> &mut [u8] {
        split_last_page(self.0.as_slice()).1
    }
}

#[cfg(not(stack_grows_up))]
fn split_last_page(slice: &mut [u8]) -> (*mut u8, &mut [u8]) {
    // Last page is at the end of stack, in case of full-descend
    let (guard, stack) = slice.split_at_mut(page_size());
    (guard.as_mut_ptr(), stack)
}

#[cfg(stack_grows_up)]
fn split_last_page(slice: &mut [u8]) -> (*mut u8, &mut [u8]) {
    let mid = slice.len() - page_size();
    let (stack, guard) = slice.split_at_mut(mid);
    (guard.as_mut_ptr(), stack)
}

#[cfg(unix)]
fn protect_page(page_ptr: *mut u8) -> bool {
    use libc::{mprotect, c_void, size_t, PROT_NONE};
    unsafe {
        mprotect(
            page_ptr    as *mut c_void,
            page_size() as size_t,
            PROT_NONE
        ) != -1
    }
}

#[cfg(windows)]
fn protect_page(page_ptr: *mut u8) -> bool {
    use libc::{VirtualProtect, c_void, SIZE_T, DWORD, LPDWORD, PAGE_NOACCESS};
    unsafe {
        let mut old_prot: DWORD = 0;
        VirtualProtect(
            page_ptr,
            page_size() as SIZE_T,
            PAGE_NOACCESS,
            &mut old_prot as LPDWORD
        ) != 0
    }
}

pub struct ScopedStack<'a>(&'a mut [u8]);

impl<'a> ScopedStack<'a> {
    pub fn new(slice: &'a mut [u8]) -> ScopedStack<'a> {
        ScopedStack(slice)
    }
}

impl<'a> Stack for ScopedStack<'a> {
    fn as_slice(&mut self) -> &mut [u8] {
        self.0
    }
}

/*
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
*/