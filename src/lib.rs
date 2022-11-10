//! A Custom memory allocator using slab and buddy allocators.
//!
//! ```
//! use memac::Allocator;
//! use core::alloc::GlobalAlloc;
//!
//! let mut alloc = Allocator::new();
//!
//! let heap_size = 32 * 1024 * 1024;
//! let layout = std::alloc::Layout::from_size_align(heap_size, memac::ALIGNMENT).unwrap();
//! let ptr = unsafe { std::alloc::alloc(layout) };
//!
//! alloc.init(ptr as usize); // Initialize the allocator.
//!
//! let layout = std::alloc::Layout::from_size_align(128, 32).unwrap();
//! let mem = unsafe { alloc.alloc(layout) }; // Allocation.
//! unsafe { alloc.dealloc(mem, layout) };    // Deallocation.
//! ```

#![no_std]

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};
use synctools::mcs::{MCSLock, MCSNode};

extern crate alloc;

mod buddy;
pub mod pager;
mod slab;

/// A custom memory allocator using slab and buddy allocators.
pub struct Allocator {
    slab: Option<MCSLock<slab::SlabAllocator>>,
    unmapf: fn(usize, usize),
}

const SIZE_64K: usize = 64 * 1024;
const MASK_64K: usize = SIZE_64K - 1;

pub const ALIGNMENT: usize = SIZE_64K;
pub const MASK: usize = !(MASK_64K);

impl Allocator {
    pub const fn new() -> Allocator {
        fn dummy(_: usize, _: usize) {}

        Allocator {
            slab: None,
            unmapf: dummy,
        }
    }

    /// Initialize allocator.
    ///
    /// - `heap_size = 2^`buddy::MAX_DEPTH` * `min_size`
    /// - `heap_end` = `heap_start` + `heap_size`
    pub fn init(&mut self, heap_start: usize) {
        assert_eq!(heap_start & MASK_64K, 0);

        let s = slab::SlabAllocator::new(heap_start);
        self.slab = Some(MCSLock::new(s));
    }

    /// Set a callback function to unmap a memory region.
    pub fn set_unmap_callback(&mut self, unmapf: fn(usize, usize)) {
        self.unmapf = unmapf;
    }

    /// Allocate a memory region.
    pub fn mem_alloc_align(&self, layout: Layout) -> Option<*mut u8> {
        let size = layout.size();
        let alignment = layout.align();

        if alignment <= 8 {
            self.mem_alloc(size)
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
        if size <= slab::MAX_SLAB_SIZE {
            let mut node = MCSNode::new();
            unsafe {
                if let Some(slab) = &self.slab {
                    slab.lock(&mut node).slab_alloc(size)
                } else {
                    None
                }
            }
        } else {
            let mut node = MCSNode::new();
            if let Some(slab) = &self.slab {
                slab.lock(&mut node).buddy.buddy_alloc(size)
            } else {
                None
            }
        }
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
                (self.unmapf)(addr, addr);
            }
        } else {
            {
                let mut node = MCSNode::new();
                if let Some(slab) = &self.slab {
                    slab.lock(&mut node).buddy.buddy_free(ptr);
                }
            }

            let start = ptr as usize;
            let end = start >> (16 + if start & MASK_64K == 0 { 0 } else { 1 });
            (self.unmapf)(start, end);
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
                null_mut()
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
                null_mut()
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

    fn init() -> (Allocator, *mut u8) {
        let mut alloc = Allocator::new();

        let heap_size = 32 * 1024 * 1024;
        let layout = std::alloc::Layout::from_size_align(heap_size, crate::ALIGNMENT).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };

        alloc.init(ptr as usize);

        (alloc, ptr)
    }

    fn free(ptr: *mut u8) {
        let heap_size = 32 * 1024 * 1024;
        let layout = std::alloc::Layout::from_size_align(heap_size, crate::ALIGNMENT).unwrap();
        unsafe { std::alloc::dealloc(ptr, layout) };
    }

    #[test]
    fn test_alloc() {
        for _ in 0..64 {
            for align in 0..=7 {
                let (alloc, ptr) = init();
                let mut v = std::vec::Vec::new();

                for i in 0..16 {
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

                free(ptr);
            }
        }
    }
}
