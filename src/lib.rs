#![no_std]

use core::alloc::{GlobalAlloc, Layout};

extern crate alloc;

mod buddy;
pub mod pager;
mod slab;

struct Allocator {}

#[global_allocator]
static GLOBAL: Allocator = Allocator {};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            slab::slab_alloc(layout)
        } else {
            buddy::buddy_alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            slab::slab_dealloc(ptr, layout)
        } else {
            buddy::buddy_dealloc(ptr, layout)
        }
    }
}

const SIZE_64K: usize = 64 * 1024;
const MASK_64K: usize = SIZE_64K - 1;

/// initialize slab allocator
/// heap_start must be aligned with 64KiB
/// heap_size must be 64KiB
pub fn init_slab(heap_start: usize, heap_size: usize) {
    assert_eq!(heap_start & MASK_64K, 0);
    assert_eq!(heap_size & MASK_64K, 0);
    slab::init(heap_start, heap_size);
}

/// initialize buddy allocator
/// heap_start must be aligned with 64KiB
pub fn init_buddy(heap_start: usize) {
    assert_eq!(heap_start & MASK_64K, 0);
    buddy::init(SIZE_64K, heap_start);
}

#[cfg(test)]
mod tests {
    //extern crate std;
    //use std::alloc::{GlobalAlloc, Layout, System};
    //use std::vec::Vec;

    #[test]
    fn test_alloc() {
        //let layout = Layout::from_size_align(32 * 1024 * 1024, 64 * 1024).unwrap();
        //let ptr = unsafe { System.alloc(layout) };
        /*
        for i in 0..8 {
            Vec::<u8>::with_capacity(1 << i);
        }*/
    }
}
