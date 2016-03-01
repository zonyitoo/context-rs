// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::io;
use std::ops::Deref;
use std::os::raw::c_void;

use sys;

/// Error type returned by stack allocation methods.
#[derive(Debug)]
pub enum StackError {
    /// Contains the maximum amount of memory allowed to be allocated as stack space.
    ExceedsMaximumSize(usize),

    /// Returned if some kind of I/O error happens during allocation.
    IoError(io::Error),
}

/// Represents some kind of stack memory.
/// Use either FixedSizeStack or ProtectedFixedSizeStack to allocate actual stack space.
#[derive(Debug)]
pub struct Stack {
    top: *mut c_void,
    bottom: *mut c_void,
}

impl Stack {
    /// Creates a representation of some stack memory (non-owning).
    #[inline]
    pub fn new(top: *mut c_void, bottom: *mut c_void) -> Stack {
        debug_assert!(top >= bottom);

        Stack {
            top: top,
            bottom: bottom,
        }
    }

    /// Returns the top of the stack from which on it grows downwards towards bottom().
    #[inline]
    pub fn top(&self) -> *mut c_void {
        self.top
    }

    /// Returns the bottom of the stack and thus it's end.
    #[inline]
    pub fn bottom(&self) -> *mut c_void {
        self.bottom
    }

    /// Returns the size of the stack between top() and bottom().
    #[inline]
    pub fn len(&self) -> usize {
        self.top as usize - self.bottom as usize
    }

    /// Returns the minimal stack size allowed by the current platform.
    #[inline]
    pub fn min_size() -> usize {
        sys::min_stack_size()
    }

    /// Returns the maximum stack size allowed by the current platform.
    #[inline]
    pub fn max_size() -> usize {
        sys::max_stack_size()
    }

    /// Returns a implementation defined default stack size.
    ///
    /// This is usually provided by the "soft stack limit"
    /// if the platform has one or is a multiple of the `min_size()`.
    #[inline]
    pub fn default_size() -> usize {
        sys::default_stack_size()
    }

    /// Allocate a new stack of `size`.
    fn allocate(mut size: usize, protected: bool) -> Result<Stack, StackError> {
        let page_size = sys::page_size();
        let min_stack_size = sys::min_stack_size();
        let max_stack_size = sys::max_stack_size();
        let add_shift = if protected {
            1
        } else {
            0
        };
        let add = page_size << add_shift;

        if size < min_stack_size {
            size = min_stack_size;
        }

        size = (size - 1) & !(page_size - 1);

        if let Some(size) = size.checked_add(add) {
            if size <= max_stack_size {
                let mut ret = sys::allocate_stack(size);

                if protected {
                    if let Ok(stack) = ret {
                        ret = sys::protect_stack(&stack);
                    }
                }

                return ret.map_err(StackError::IoError);
            }
        }

        Err(StackError::ExceedsMaximumSize(max_stack_size - add))
    }
}

/// A simple implementation of `Stack`.
///
/// Allocates stack space using virtual memory, whose pages will
/// only be mapped to physical memory if they are used.
///
/// It is recommended to use `ProtectedFixedSizeStack` instead.
#[derive(Debug)]
pub struct FixedSizeStack(Stack);

impl FixedSizeStack {
    /// Allocate a new stack of **at least** `size` bytes + one additional guard page.
    ///
    /// `size` is rounded up to a multiple of the size of a memory page and
    /// does not include the size of the guard page itself.
    pub fn new(size: usize) -> Result<FixedSizeStack, StackError> {
        Stack::allocate(size, false).map(FixedSizeStack)
    }
}

impl Deref for FixedSizeStack {
    type Target = Stack;

    fn deref(&self) -> &Stack {
        &self.0
    }
}

impl Default for FixedSizeStack {
    fn default() -> FixedSizeStack {
        FixedSizeStack::new(Stack::default_size())
            .unwrap_or_else(|err| panic!("Failed to allocate FixedSizeStack with {:?}", err))
    }
}

impl Drop for FixedSizeStack {
    fn drop(&mut self) {
        sys::deallocate_stack(self.0.bottom(), self.0.len());
    }
}

/// A more secure, but a bit slower implementation of `Stack` compared to `FixedSizeStack`.
///
/// Allocates stack space using virtual memory, whose pages will
/// only be mapped to physical memory if they are used.
///
/// The additional guard page is made protected and inaccessible.
/// Now if a stack overflow occurs it should (hopefully) hit this guard page and
/// will cause a segmentation fault instead letting the memory being silently overwritten.
///
/// It is recommended to use this class in general to create stack memory.
#[derive(Debug)]
pub struct ProtectedFixedSizeStack(Stack);

impl ProtectedFixedSizeStack {
    /// Allocate a new stack of **at least** `size` bytes + one additional guard page.
    ///
    /// `size` is rounded up to a multiple of the size of a memory page and
    /// does not include the size of the guard page itself.
    pub fn new(size: usize) -> Result<ProtectedFixedSizeStack, StackError> {
        Stack::allocate(size, true).map(ProtectedFixedSizeStack)
    }
}

impl Deref for ProtectedFixedSizeStack {
    type Target = Stack;

    fn deref(&self) -> &Stack {
        &self.0
    }
}

impl Default for ProtectedFixedSizeStack {
    fn default() -> ProtectedFixedSizeStack {
        ProtectedFixedSizeStack::new(Stack::default_size()).unwrap_or_else(|err| {
            panic!("Failed to allocate ProtectedFixedSizeStack with {:?}", err)
        })
    }
}

impl Drop for ProtectedFixedSizeStack {
    fn drop(&mut self) {
        let page_size = sys::page_size();
        let guard = (self.0.bottom() as usize - page_size) as *mut c_void;
        let size_with_guard = self.0.len() + page_size;
        sys::deallocate_stack(guard, size_with_guard);
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::write_bytes;

    use super::*;
    use sys;

    #[test]
    fn stack_size_too_small() {
        let stack = FixedSizeStack::new(0).unwrap();
        assert_eq!(stack.len(), sys::min_stack_size());

        unsafe { write_bytes(stack.bottom() as *mut u8, 0x1d, stack.len()) };

        let stack = ProtectedFixedSizeStack::new(0).unwrap();
        assert_eq!(stack.len(), sys::min_stack_size());

        unsafe { write_bytes(stack.bottom() as *mut u8, 0x1d, stack.len()) };
    }

    #[test]
    fn stack_size_too_large() {
        let stack_size = sys::max_stack_size() & !(sys::page_size() - 1);

        match FixedSizeStack::new(stack_size) {
            Err(StackError::ExceedsMaximumSize(..)) => panic!(),
            _ => {}
        }

        let stack_size = stack_size + 1;

        match FixedSizeStack::new(stack_size) {
            Err(StackError::ExceedsMaximumSize(..)) => {}
            _ => panic!(),
        }
    }
}
