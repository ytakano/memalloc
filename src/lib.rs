#![no_std]

use core::{alloc::{GlobalAlloc, Layout}, ptr::null_mut};

extern crate alloc;

pub mod pager;
mod slab;
mod buddy;

struct Allocator {}

#[global_allocator]
static GLOBAL: Allocator = Allocator {};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            slab::slab_alloc(layout)
        } else {
            /*
            match BUDDY_ALLOC.mem_alloc(layout.size()) {
                Some(addr) => addr,
                None => {
                    handle_alloc_error(layout);
                }
            }*/
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if slab::MAX_SLAB_SIZE >= layout.size() {
            slab::slab_dealloc(ptr, layout)
        } else {
            //BUDDY_ALLOC.mem_free(ptr);
        }
    }
}

pub fn init_slab(heap_start : usize, heap_size : usize) {
    unsafe { slab::init(heap_start, heap_size) };
}

#[cfg(test)]
mod tests {
    extern crate std;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
