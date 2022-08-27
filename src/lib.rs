//! A Custom memory allocator using slab and buddy allocators.
//!
//! ```
//! use memalloc::Allocator;
//! use core::alloc::GlobalAlloc;
//!
//! let mut alloc = Allocator::new();
//!
//! let heap_size = 32 * 1024 * 1024;
//! let layout = std::alloc::Layout::from_size_align(heap_size, memalloc::ALIGNMENT).unwrap();
//! let ptr1 = unsafe { std::alloc::alloc(layout) };
//! let ptr2 = unsafe { std::alloc::alloc(layout) };
//!
//! alloc.init_buddy(ptr1 as usize);           // Set a region for the buddy allocator.
//! alloc.init_slab(ptr2 as usize, heap_size); // Set a region for the slab allocator.
//!
//! let layout = std::alloc::Layout::from_size_align(128, 32).unwrap();
//! let mem = unsafe { alloc.alloc(layout) }; // Allocation.
//! unsafe { alloc.dealloc(mem, layout) };    // Deallocation.
//! ```

#![no_std]

use alloc::alloc::handle_alloc_error;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null,
};
use synctools::mcs::{MCSLock, MCSNode};

extern crate alloc;

mod buddy;
pub mod pager;
mod slab;

/// A custom memory allocator using slab and buddy allocators.
pub struct Allocator {
    buddy: Option<MCSLock<buddy::BuddyAlloc>>,
    slab: Option<MCSLock<slab::SlabAllocator>>,
    unmapf: *const (),
}

const SIZE_64K: usize = 64 * 1024;
const MASK_64K: usize = SIZE_64K - 1;

pub const ALIGNMENT: usize = SIZE_64K;
pub const MASK: usize = !(MASK_64K);

impl Allocator {
    pub const fn new() -> Allocator {
        Allocator {
            buddy: None,
            slab: None,
            unmapf: null(),
        }
    }

    /// Initialize slab allocator.
    /// `heap_start` must be aligned with 64KiB, and
    /// `heap_size` must be 64KiB.
    pub fn init_slab(&mut self, heap_start: usize, heap_size: usize) {
        assert_eq!(heap_start & MASK_64K, 0);
        assert_eq!(heap_size & MASK_64K, 0);

        let mut s = slab::SlabAllocator::new();
        s.init(heap_start, heap_size);
        self.slab = Some(MCSLock::new(s));
    }

    /// Initialize buddy allocator.
    /// `heap_start` must be aligned with 64KiB.
    ///
    /// - `heap_end` = `heap_start` + 2^`buddy::MAX_DEPTH` * `min_size`
    /// - `heap_size` = `heap_end` - `heap_size`
    pub fn init_buddy(&mut self, heap_start: usize) {
        assert_eq!(heap_start & MASK_64K, 0);
        let b = buddy::BuddyAlloc::new(SIZE_64K, heap_start);
        self.buddy = Some(MCSLock::new(b));
    }

    /// set a callback function to unmap a memory region.
    pub fn set_unmap_callback(&mut self, unmapf: fn(usize, usize)) {
        self.unmapf = unmapf as *const ();
    }

    /// Allocate a memory region.
    pub fn mem_alloc_align(&self, layout: Layout) -> Option<*mut u8> {
        let size = layout.size();
        let alignment = layout.align();

        if alignment <= 8 {
            if let Some(ptr) = self.mem_alloc(size) {
                Some(ptr)
            } else {
                None
            }
        } else {
            let align_1 = alignment - 1;
            let size = size + align_1 + 8;
            if let Some(ptr) = self.mem_alloc(size) {
                let addr = ((ptr as usize) + align_1 + 8) & !align_1;
                let result = addr as *mut u8;
                let ptr_to_orig = (addr - 8) as *mut u64;

                unsafe { *ptr_to_orig = ptr as u64 };

                Some(result)
            } else {
                None
            }
        }
    }

    /// Deallocate a memory region.
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer returned by `mem_alloc`.
    pub unsafe fn mem_free_align(&mut self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        let alignment = layout.align();

        if alignment <= 8 {
            self.mem_free(ptr, size)
        } else {
            let addr = ptr as usize;
            let ptr_to_orig = (addr - 8) as *mut u64;
            let ptr = (*ptr_to_orig) as *mut u8;
            let size = size + alignment - 1 + 8;
            self.mem_free(ptr, size);
        }
    }

    fn mem_alloc(&self, size: usize) -> Option<*mut u8> {
        let result;
        if size <= slab::MAX_SLAB_SIZE {
            let mut node = MCSNode::new();
            result = unsafe {
                if let Some(slab) = &self.slab {
                    slab.lock(&mut node).slab_alloc(size)
                } else {
                    None
                }
            };
        } else {
            let mut node = MCSNode::new();
            result = if let Some(buddy) = &self.buddy {
                buddy.lock(&mut node).buddy_alloc(size)
            } else {
                None
            }
        }
        result
    }

    unsafe fn mem_free(&self, ptr: *mut u8, size: usize) {
        if slab::MAX_SLAB_SIZE >= size {
            let result;
            {
                let mut node = MCSNode::new();
                result = if let Some(slab) = &self.slab {
                    slab.lock(&mut node).slab_dealloc(ptr)
                } else {
                    return;
                }
            }
            if let Some(addr) = result {
                if !self.unmapf.is_null() {
                    let unmapf = core::mem::transmute::<*const (), fn(usize, usize)>(self.unmapf);
                    unmapf(addr, addr);
                }
            }
        } else {
            {
                let mut node = MCSNode::new();
                if let Some(buddy) = &self.buddy {
                    buddy.lock(&mut node).buddy_free(ptr);
                }
            }

            if !self.unmapf.is_null() {
                let unmapf = core::mem::transmute::<*const (), fn(usize, usize)>(self.unmapf);
                let start = ptr as usize;
                let end = start >> (16 + if start & MASK_64K == 0 { 0 } else { 1 });
                unmapf(start, end);
            }
        }
    }
}

//#[global_allocator]
//static GLOBAL: Allocator = Allocator {};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let alignment = layout.align();

        if alignment <= 8 {
            if let Some(ptr) = self.mem_alloc(size) {
                ptr
            } else {
                handle_alloc_error(layout)
            }
        } else {
            let align_1 = alignment - 1;
            let size = size + align_1 + 8;
            if let Some(ptr) = self.mem_alloc(size) {
                let addr = ((ptr as usize) + align_1 + 8) & !align_1;
                let result = addr as *mut u8;
                let ptr_to_orig = (addr - 8) as *mut u64;

                *ptr_to_orig = ptr as u64;

                result
            } else {
                handle_alloc_error(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        let alignment = layout.align();

        if alignment <= 8 {
            self.mem_free(ptr, size)
        } else {
            let addr = ptr as usize;
            let ptr_to_orig = (addr - 8) as *mut u64;
            let ptr = (*ptr_to_orig) as *mut u8;
            let size = size + alignment - 1 + 8;
            self.mem_free(ptr, size);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::alloc::GlobalAlloc;
    use std::println;

    use crate::Allocator;

    fn init() -> (Allocator, *mut u8, *mut u8) {
        let mut alloc = Allocator::new();

        let heap_size = 32 * 1024 * 1024;
        let layout = std::alloc::Layout::from_size_align(heap_size, crate::ALIGNMENT).unwrap();
        let ptr1 = unsafe { std::alloc::alloc(layout) };
        let ptr2 = unsafe { std::alloc::alloc(layout) };

        alloc.init_buddy(ptr1 as usize);
        alloc.init_slab(ptr2 as usize, heap_size);

        (alloc, ptr1, ptr2)
    }

    fn free(ptr1: *mut u8, ptr2: *mut u8) {
        let heap_size = 32 * 1024 * 1024;
        let layout = std::alloc::Layout::from_size_align(heap_size, crate::ALIGNMENT).unwrap();
        unsafe { std::alloc::dealloc(ptr1, layout) };
        unsafe { std::alloc::dealloc(ptr2, layout) };
    }

    #[test]
    fn test_alloc() {
        for _ in 0..64 {
            for align in 0..=7 {
                let (alloc, ptr1, ptr2) = init();
                let mut v = std::vec::Vec::new();

                for i in 0..15 {
                    let size = 4 << i;
                    for j in 0..16 {
                        let size = size + (rand::random::<usize>() % size);
                        let layout = std::alloc::Layout::from_size_align(size, 1 << align).unwrap();

                        println!("allocate: {i}, {j}, layout = {:?}", layout);

                        let mem = unsafe { alloc.alloc(layout) };
                        v.push((mem, layout));

                        // must be aligned
                        assert_eq!(mem as usize % 1 << align, 0);
                    }
                }

                for (mem, layout) in v {
                    println!("deallocate: layout = {:?}", layout);
                    unsafe { alloc.dealloc(mem, layout) };
                }

                free(ptr1, ptr2);
            }
        }
    }
}
