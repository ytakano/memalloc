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

pub struct Allocator {
    buddy: Option<MCSLock<buddy::BuddyAlloc>>,
    slab: Option<MCSLock<slab::SlabAllocator>>,
    unmapf: *const (),
}

const SIZE_64K: usize = 64 * 1024;
const MASK_64K: usize = SIZE_64K - 1;

pub const ALIGNMENT: usize = SIZE_64K;

impl Allocator {
    pub const fn new() -> Allocator {
        Allocator {
            buddy: None,
            slab: None,
            unmapf: null(),
        }
    }

    /// initialize slab allocator
    /// heap_start must be aligned with 64KiB
    /// heap_size must be 64KiB
    pub fn init_slab(&mut self, heap_start: usize, heap_size: usize) {
        assert_eq!(heap_start & MASK_64K, 0);
        assert_eq!(heap_size & MASK_64K, 0);

        let mut s = slab::SlabAllocator::new();
        s.init(heap_start, heap_size);
        self.slab = Some(MCSLock::new(s));
    }

    /// initialize buddy allocator
    /// heap_start must be aligned with 64KiB
    ///
    /// heap_end = heap_start + 2^buddy::MAX_DEPTH * min_size
    /// heap_size = heap_end - heap_size
    pub fn init_buddy(&mut self, heap_start: usize) {
        assert_eq!(heap_start & MASK_64K, 0);
        let b = buddy::BuddyAlloc::new(SIZE_64K, heap_start);
        self.buddy = Some(MCSLock::new(b));
    }

    pub fn set_unmap_callback(&mut self, unmapf: fn(usize, usize)) {
        self.unmapf = unmapf as *const ();
    }
}

//#[global_allocator]
//static GLOBAL: Allocator = Allocator {};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result;
        if slab::MAX_SLAB_SIZE >= layout.size() {
            let mut node = MCSNode::new();
            result = self
                .slab
                .as_ref()
                .expect("slab allocator is not yet initialized")
                .lock(&mut node)
                .slab_alloc(layout);
        } else {
            let mut node = MCSNode::new();
            result = self
                .buddy
                .as_ref()
                .expect("buddy allocator is not yet initialized")
                .lock(&mut node)
                .mem_alloc(layout.size());
        };

        if let Some(ptr) = result {
            ptr
        } else {
            handle_alloc_error(layout);
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            let result;
            {
                let mut node = MCSNode::new();
                result = self
                    .slab
                    .as_ref()
                    .expect("slab allocator is not yet initialized")
                    .lock(&mut node)
                    .slab_dealloc(ptr, layout);
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
                self.buddy
                    .as_ref()
                    .expect("buddy allocator is not yet initialized")
                    .lock(&mut node)
                    .mem_free(ptr);
            }

            if !self.unmapf.is_null() {
                let unmapf = core::mem::transmute::<*const (), fn(usize, usize)>(self.unmapf);
                let start = ptr as usize;
                let end = start >> 16 + if start & MASK_64K == 0 { 0 } else { 1 };
                unmapf(start, end);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::alloc::GlobalAlloc;

    use crate::Allocator;

    fn init() -> Allocator {
        let mut alloc = Allocator::new();

        let heap_size = 32 * 1024 * 1024;
        let layout = std::alloc::Layout::from_size_align(heap_size, crate::ALIGNMENT).unwrap();
        let ptr1 = unsafe { std::alloc::alloc(layout) };
        let ptr2 = unsafe { std::alloc::alloc(layout) };

        alloc.init_buddy(ptr1 as usize);
        alloc.init_slab(ptr2 as usize, heap_size);

        alloc
    }

    #[test]
    fn test_alloc() {
        let alloc = init();
        let mut v = std::vec::Vec::new();
        for i in 0..15 {
            let layout = std::alloc::Layout::from_size_align(8 << i, 4).unwrap();
            for _ in 0..128 {
                let mem = unsafe { alloc.alloc(layout) };
                v.push((mem, layout));
            }
        }

        for (mem, layout) in v {
            unsafe { alloc.dealloc(mem, layout) };
        }
    }
}
