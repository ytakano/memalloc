#![no_std]

use alloc::alloc::handle_alloc_error;
use core::alloc::{GlobalAlloc, Layout};
use synctools::mcs::MCSLock;

extern crate alloc;

mod buddy;
pub mod pager;
mod slab;

pub struct Allocator {
    buddy: Option<MCSLock<buddy::BuddyAlloc>>,
    slab: Option<MCSLock<slab::SlabAllocator>>
}

const SIZE_64K: usize = 64 * 1024;
const MASK_64K: usize = SIZE_64K - 1;

pub const ALIGNMENT: usize = SIZE_64K;

impl Allocator {
    pub const fn new() -> Allocator {
        Allocator {
            buddy: None,
            slab: None
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
}

//#[global_allocator]
//static GLOBAL: Allocator = Allocator {};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            self.slab
                .as_ref()
                .expect("slab allocator is not yet initialized")
                .lock().slab_alloc(layout)
        } else {
            match self
                .buddy
                .as_ref()
                .expect("buddy allocator is not yet initialized")
                .lock()
                .mem_alloc(layout.size())
            {
                Some(addr) => addr,
                None => handle_alloc_error(layout),
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            self.slab
                .as_ref()
                .expect("slab allocator is not yet initialized")
                .lock().slab_dealloc(ptr, layout)
        } else {
            self.buddy
                .as_ref()
                .expect("buddy allocator is not yet initialized")
                .lock()
                .mem_free(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::Allocator;
    //use std::alloc::{GlobalAlloc, Layout, System};
    //use std::vec::Vec;

    #[test]
    fn test_alloc() {
        Allocator::new();
    }
}
