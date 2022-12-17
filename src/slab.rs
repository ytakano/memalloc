use crate::{MemAlloc, SIZE_64K};
use core::ptr::null_mut;

pub(crate) const MAX_SLAB_SIZE: usize = 65512 - 8;

pub(crate) struct SlabAllocator<PAGEALLOC: MemAlloc> {
    pub(crate) page_alloc: PAGEALLOC,

    slab16_partial: *mut Slab16,
    slab32_partial: *mut Slab32,
    slab64_partial: *mut Slab64,
    slab128_partial: *mut Slab128,
    slab256_partial: *mut Slab256,
    slab512_partial: *mut Slab512,
    slab1024_partial: *mut Slab1024,
    slab2040_partial: *mut Slab2040,
    slab4088_partial: *mut Slab4088,
    slab8184_partial: *mut Slab8184,
    slab16376_partial: *mut Slab16376,
    slab32752_partial: *mut Slab32752,
    slab65512_partial: *mut Slab65512, // always null

    slab16_full: *mut Slab16,
    slab32_full: *mut Slab32,
    slab64_full: *mut Slab64,
    slab128_full: *mut Slab128,
    slab256_full: *mut Slab256,
    slab512_full: *mut Slab512,
    slab1024_full: *mut Slab1024,
    slab2040_full: *mut Slab2040,
    slab4088_full: *mut Slab4088,
    slab8184_full: *mut Slab8184,
    slab16376_full: *mut Slab16376,
    slab32752_full: *mut Slab32752,
    slab65512_full: *mut Slab65512,
}

unsafe fn alloc_memory<PAGEALLOC: MemAlloc, SLAB: Slab>(
    page_alloc: &mut PAGEALLOC,
    slab_partial: &mut *mut SLAB,
    slab_full: &mut *mut SLAB,
) -> Option<*mut u8> {
    let slab_partial_top = slab_partial;
    let slab_partial = *slab_partial_top;

    let slab_full_top = slab_full;
    let slab_full = *slab_full_top;

    match slab_partial.as_mut() {
        Some(partial) => {
            let ret = partial.alloc(); // Allocate a memory region.

            if partial.is_full() {
                if let Some(next) = partial.next().as_mut() {
                    next.set_prev(null_mut());
                }

                *slab_partial_top = partial.next();

                if let Some(full) = slab_full.as_mut() {
                    full.set_prev(slab_partial);
                }

                partial.set_next(slab_full);
            }

            Some(ret)
        }
        None => {
            if let Some(addr) = page_alloc.alloc(SIZE_64K) {
                let slab_ptr = addr as *mut SLAB;

                if let Some(slab) = slab_ptr.as_mut() {
                    slab.init();

                    let ret = slab.alloc();

                    if slab.is_full() {
                        // for only Slab65512
                        if let Some(full) = slab_full.as_mut() {
                            full.set_prev(slab_ptr);
                        }

                        slab.set_next(slab_full);
                        *slab_full_top = slab_ptr;
                    } else {
                        *slab_partial_top = slab_ptr;
                    }

                    Some(ret)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

unsafe fn dealloc_memory<PAGEALLOC: MemAlloc, SLAB: Slab>(
    ptr: *mut u8,
    addr_slab: usize,
    page_alloc: &mut PAGEALLOC,
    slab_partial: &mut *mut SLAB,
    slab_full: &mut *mut SLAB,
) -> Option<usize> {
    if let Some(slab) = (addr_slab as *mut SLAB).as_mut() {
        let is_full = slab.is_full();
        slab.free(ptr);
        if is_full {
            if let Some(prev) = slab.prev().as_mut() {
                prev.set_next(slab.next());
            } else {
                *slab_full = slab.next();
            }

            if let Some(next) = slab.next().as_mut() {
                next.set_prev(slab.prev());
            }

            if slab.is_empty() {
                page_alloc.free(addr_slab as *mut u8);
                Some(addr_slab) // Should unmap this page.
            } else {
                if let Some(partial) = slab_partial.as_mut() {
                    partial.set_prev(slab);
                    slab.set_next(partial);
                } else {
                    slab.set_next(null_mut());
                }

                slab.set_prev(null_mut());
                *slab_partial = slab;

                None
            }
        } else if slab.is_empty() {
            if let Some(prev) = slab.prev().as_mut() {
                prev.set_next(slab.next());
            } else {
                *slab_partial = slab.next();
            }

            if let Some(next) = slab.next().as_mut() {
                next.set_prev(slab.prev());
            }

            page_alloc.free(addr_slab as *mut u8);
            Some(addr_slab) // Should unmap this page.
        } else {
            None
        }
    } else {
        None
    }
}

impl<PAGEALLOC: MemAlloc> SlabAllocator<PAGEALLOC> {
    pub(crate) unsafe fn slab_alloc(&mut self, size: usize) -> Option<*mut u8> {
        let n = (size as u64 + 8 - 1).leading_zeros();

        match n {
            61 | 60 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab16_partial,
                &mut self.slab16_full,
            ),
            59 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab32_partial,
                &mut self.slab32_full,
            ),
            58 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab64_partial,
                &mut self.slab64_full,
            ),
            57 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab128_partial,
                &mut self.slab128_full,
            ),
            56 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab256_partial,
                &mut self.slab256_full,
            ),
            55 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab512_partial,
                &mut self.slab512_full,
            ),
            54 => alloc_memory(
                &mut self.page_alloc,
                &mut self.slab1024_partial,
                &mut self.slab1024_full,
            ),
            _ => {
                if size <= 4088 - 16 {
                    if size <= 2040 - 16 {
                        // Slab2040
                        alloc_memory(
                            &mut self.page_alloc,
                            &mut self.slab2040_partial,
                            &mut self.slab2040_full,
                        )
                    } else {
                        // Slab4088
                        alloc_memory(
                            &mut self.page_alloc,
                            &mut self.slab4088_partial,
                            &mut self.slab4088_full,
                        )
                    }
                } else if size <= 16376 - 16 {
                    if size <= 8184 - 16 {
                        // Slab8184
                        alloc_memory(
                            &mut self.page_alloc,
                            &mut self.slab8184_partial,
                            &mut self.slab8184_full,
                        )
                    } else {
                        // Slab16376
                        alloc_memory(
                            &mut self.page_alloc,
                            &mut self.slab16376_partial,
                            &mut self.slab16376_full,
                        )
                    }
                } else if size <= 32752 - 16 {
                    // Slab32752
                    alloc_memory(
                        &mut self.page_alloc,
                        &mut self.slab32752_partial,
                        &mut self.slab32752_full,
                    )
                } else if size <= 65512 - 8 {
                    // Slab65512
                    alloc_memory(
                        &mut self.page_alloc,
                        &mut self.slab65512_partial,
                        &mut self.slab65512_full,
                    )
                } else {
                    None
                }
            }
        }
    }

    /// Return a 64KiB page address if page should be unmapped.
    pub(crate) unsafe fn slab_dealloc(&mut self, ptr: *mut u8) -> Option<usize> {
        let addr_slab = *((ptr as usize - 8) as *const u64);
        let size = *((addr_slab + 65532) as *const u32);

        /*
                driver::uart::puts("dealloc:\n");
                driver::uart::puts("  ptr: 0x");
                driver::uart::hex(ptr as u64);
                driver::uart::puts("\n");
                driver::uart::puts("  addr_slab: 0x");
                driver::uart::hex(addr_slab);
                driver::uart::puts("\n");
                driver::uart::puts("  size: ");
                driver::uart::decimal(size as u64);
                driver::uart::puts("\n");
        */
        match size {
            16 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab16_partial,
                &mut self.slab16_full,
            ),
            32 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab32_partial,
                &mut self.slab32_full,
            ),
            64 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab64_partial,
                &mut self.slab64_full,
            ),
            128 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab128_partial,
                &mut self.slab128_full,
            ),
            256 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab256_partial,
                &mut self.slab256_full,
            ),
            512 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab512_partial,
                &mut self.slab512_full,
            ),
            1024 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab1024_partial,
                &mut self.slab1024_full,
            ),
            2040 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab2040_partial,
                &mut self.slab2040_full,
            ),
            4088 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab4088_partial,
                &mut self.slab4088_full,
            ),
            8184 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab8184_partial,
                &mut self.slab8184_full,
            ),
            16376 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab16376_partial,
                &mut self.slab16376_full,
            ),
            32752 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab32752_partial,
                &mut self.slab32752_full,
            ),
            65512 => dealloc_memory(
                ptr,
                addr_slab as usize,
                &mut self.page_alloc,
                &mut self.slab65512_partial,
                &mut self.slab65512_full,
            ),
            _ => None,
        }
    }

    pub(crate) fn new(addr: usize, size: usize) -> Self {
        Self {
            page_alloc: PAGEALLOC::new(addr, size),
            slab16_partial: null_mut(),
            slab32_partial: null_mut(),
            slab64_partial: null_mut(),
            slab128_partial: null_mut(),
            slab256_partial: null_mut(),
            slab512_partial: null_mut(),
            slab1024_partial: null_mut(),
            slab2040_partial: null_mut(),
            slab4088_partial: null_mut(),
            slab8184_partial: null_mut(),
            slab16376_partial: null_mut(),
            slab32752_partial: null_mut(),
            slab65512_partial: null_mut(),
            slab16_full: null_mut(),
            slab32_full: null_mut(),
            slab64_full: null_mut(),
            slab128_full: null_mut(),
            slab256_full: null_mut(),
            slab512_full: null_mut(),
            slab1024_full: null_mut(),
            slab2040_full: null_mut(),
            slab4088_full: null_mut(),
            slab8184_full: null_mut(),
            slab16376_full: null_mut(),
            slab32752_full: null_mut(),
            slab65512_full: null_mut(),
        }
    }
}

trait Slab {
    fn alloc(&mut self) -> *mut u8;
    fn free(&mut self, ptr: *mut u8);
    fn is_full(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn init(&mut self);
    fn next(&self) -> *mut Self;
    fn prev(&self) -> *mut Self;
    fn set_next(&mut self, next: *mut Self);
    fn set_prev(&mut self, prev: *mut Self);
    // fn print(&self);
}

macro_rules! SlabSmall {
    ($id:ident, $n:expr, $shift:expr, $l1val:expr, $l2val:expr, $size:expr) => {
        #[repr(C)]
        struct $id {
            buf: [u8; 65536 - 32 - 8 * $n],
            l1_bitmap: u64,
            l2_bitmap: [u64; $n],
            prev: *mut $id,
            next: *mut $id,
            num: u32,
            size: u32,
        }

        impl Slab for $id {
            fn next(&self) -> *mut Self {
                self.next
            }

            fn set_next(&mut self, next: *mut Self) {
                self.next = next;
            }

            fn prev(&self) -> *mut Self {
                self.prev
            }

            fn set_prev(&mut self, prev: *mut Self) {
                self.prev = prev;
            }

            // +------------------+
            // | pointer to slab  |
            // |    (8 bytes)     |
            // +------------------+ <- return value
            // |       data       |
            // | (size - 8 bytes) |
            // |                  |
            /// allocate a memory region whose size is self.size - 8 bytes
            fn alloc(&mut self) -> *mut u8 {
                let idx1 = (!self.l1_bitmap).leading_zeros() as usize;
                let idx2 = (!self.l2_bitmap[idx1]).leading_zeros() as usize;

                self.l2_bitmap[idx1] |= 1 << (63 - idx2);
                if self.l2_bitmap[idx1] == !0 {
                    self.l1_bitmap |= 1 << (63 - idx1);
                }

                let size = self.size as usize;
                let idx = idx1 * size * 64 + idx2 * size;

                if idx >= 65536 - 32 - 8 * $n {
                    panic!("allocation error");
                }

                let ptr = &mut (self.buf[idx]) as *mut u8;
                let ptr64 = ptr as *mut usize;

                // first 64 bits points the slab
                unsafe {
                    *ptr64 = self as *mut $id as usize;
                }

                self.num += 1;

                &mut (self.buf[idx + 8]) as *mut u8
            }

            /// deallocate the memory region pointed by ptr which is returned by alloc
            fn free(&mut self, ptr: *mut u8) {
                let addr = ptr as usize - 8;
                let org = self as *mut $id as usize;
                let len = addr - org;
                let idx = (len >> $shift) as usize;

                let idx1 = idx >> 6; // divide by 64
                let idx2 = idx & 0b111111;

                self.l1_bitmap &= !(1 << (63 - idx1));
                self.l2_bitmap[idx1] &= !(1 << (63 - idx2));
                self.num -= 1;
            }

            fn is_full(&self) -> bool {
                self.l1_bitmap == !0
            }

            fn is_empty(&self) -> bool {
                self.num == 0
            }

            fn init(&mut self) {
                self.l1_bitmap = $l1val;
                for it in self.l2_bitmap.iter_mut() {
                    *it = 0;
                }
                self.l2_bitmap[$n - 1] = $l2val;
                self.prev = null_mut();
                self.next = null_mut();
                self.num = 0;
                self.size = $size;
            }

            // fn print(&self) {
            //     driver::uart::puts("num: ");
            //     driver::uart::decimal(self.num as u64);
            //     driver::uart::puts("\nL1 bitmap: 0x");
            //     driver::uart::hex(self.l1_bitmap);
            //     driver::uart::puts("\n");

            //     driver::uart::puts("L2 bitmap:\n");
            //     let mut i = 0;
            //     for v in self.l2_bitmap.iter() {
            //         driver::uart::puts(" 0x");
            //         driver::uart::hex(*v);
            //         if i == 3 {
            //             driver::uart::puts("\n");
            //             i = 0;
            //         } else {
            //             i += 1;
            //         }
            //     }

            //     match unsafe { self.next.as_ref() } {
            //         Some(next) => {
            //             driver::uart::puts("\n");
            //             next.print();
            //         }
            //         None => {}
            //     }
            // }
        }
    };
}

// l1_bitmap = 0 (initial value)
// l2_bitmap[63] = 0xFFFF FFFF | 0b11 << 32 (initial value)
// size = 16
SlabSmall!(Slab16, 64, 4, 0, 0xFFFFFFFF | (0b11 << 32), 16);

// l1_bitmap = 0xFFFF FFFF (initial value)
// l2_bitmap[31] = 0b111111111 (initial value)
// size = 32
SlabSmall!(Slab32, 32, 5, 0xFFFFFFFF, 0b111111111, 32);

// l1_bitmap = 0xFFFF FFFF FFFF (initial value)
// l2_bitmap[15] = 0b111 (initial value)
// size = 64
SlabSmall!(Slab64, 16, 6, 0xFFFFFFFFFFFF, 0b111, 64);

// l1_bitmap = 0xFFFF FFFF FFFF FF (initial value)
// l2_bitmap[7] = 0b1 (initial value)
// size = 128
SlabSmall!(Slab128, 8, 7, 0xFFFFFFFFFFFFFF, 1, 128);

// l1_bitmap = 0xFFFF FFFF FFFF FFF (initial value)
// l2_bitmap[3] = 0b1 (initial value)
// size = 256
SlabSmall!(Slab256, 4, 8, 0xFFFFFFFFFFFFFFF, 1, 256);

// l1_bitmap = 0x3FFF FFFF FFFF FFFF (initial value)
// l2_bitmap[1] = 0b1 (initial value)
// size = 512
SlabSmall!(Slab512, 2, 9, 0x3FFFFFFFFFFFFFFF, 1, 512);

// l1_bitmap = 0x7FFF FFFF FFFF FFFF (initial value)
// l2_bitmap[0] = 0b1 (initial value)
// size = 1024
SlabSmall!(Slab1024, 1, 10, 0x7FFFFFFFFFFFFFFF, 1, 1024);

#[repr(C)]
struct SlabMemory {
    idx1: usize,
    slab: usize,
}

macro_rules! SlabLarge {
    ($id:ident, $l1val:expr, $size:expr) => {
        #[repr(C)]
        struct $id {
            buf: [u8; 65504],
            prev: *mut $id,
            next: *mut $id,
            l1_bitmap: u64,
            num: u32,
            size: u32,
        }

        impl Slab for $id {
            fn next(&self) -> *mut Self {
                self.next
            }

            fn set_next(&mut self, next: *mut Self) {
                self.next = next;
            }

            fn prev(&self) -> *mut Self {
                self.prev
            }

            fn set_prev(&mut self, prev: *mut Self) {
                self.prev = prev;
            }

            // +-------------------+
            // |       index       |
            // |     (8 bytes)     |
            // +-------------------+
            // |  pointer to slab  |
            // |     (8 bytes)     |
            // +-------------------+ <- return value
            // |       data        |
            // | (size - 16 bytes) |
            // |                   |
            /// allocate a memory region whose size is self.size - 16 bytes
            fn alloc(&mut self) -> *mut u8 {
                let idx1 = (!self.l1_bitmap).leading_zeros() as usize;
                self.l1_bitmap |= 1 << (63 - idx1);

                let idx = idx1 * self.size as usize;
                let ptr = &mut (self.buf[idx]) as *mut u8;
                let mem = ptr as *mut SlabMemory;

                // first 128 bits contain meta information
                unsafe {
                    (*mem).idx1 = idx1;
                    (*mem).slab = self as *mut $id as usize;
                }

                self.num += 1;

                &mut (self.buf[idx + 16]) as *mut u8
            }

            /// deallocate the memory region pointed by ptr which is returned by alloc
            fn free(&mut self, ptr: *mut u8) {
                let addr = ptr as usize;
                let idx1 = unsafe { *((addr - 16) as *mut usize) };

                self.l1_bitmap &= !(1 << (63 - idx1));
                self.num -= 1;
            }

            fn is_full(&self) -> bool {
                self.l1_bitmap == !0
            }

            fn is_empty(&self) -> bool {
                self.num == 0
            }

            fn init(&mut self) {
                self.prev = null_mut();
                self.next = null_mut();
                self.l1_bitmap = $l1val;
                self.size = $size;
                self.num = 0;
            }

            // fn print(&self) {
            //     driver::uart::puts("L1 bitmap: 0x");
            //     driver::uart::hex(self.l1_bitmap);
            //     driver::uart::puts("\n");

            //     match unsafe { self.next.as_ref() } {
            //         Some(next) => {
            //             driver::uart::puts("\n");
            //             next.print();
            //         }
            //         None => {}
            //     }
            // }
        }
    };
}

// l1_bitmap = 0xFFFF FFFF (initial value)
// size = 2040
SlabLarge!(Slab2040, 0xFFFFFFFF, 2040);

// l1_bitmap = 0xFFFF FFFF FFFF (initial value)
// size = 4088
SlabLarge!(Slab4088, 0xFFFFFFFFFFFF, 4088);

// l1_bitmap = 0xFFFF FFFF FFFF FF (initial value)
// size = 8184
SlabLarge!(Slab8184, 0xFFFFFFFFFFFFFF, 8184);

// l1_bitmap = 0xFFFF FFFF FFFF FFF (initial value)
// size = 16376
SlabLarge!(Slab16376, 0xFFFFFFFFFFFFFFF, 16376);

// l1_bitmap = 0x3FFF FFFF FFFF FFFF (initial value)
// size = 32752
SlabLarge!(Slab32752, 0x3FFFFFFFFFFFFFFF, 32752);

#[repr(C)]
struct Slab65512 {
    buf: [u8; 65512],
    prev: *mut Slab65512,
    next: *mut Slab65512,
    num: u32,
    size: u32, // must be 65512
}

impl Slab for Slab65512 {
    fn next(&self) -> *mut Self {
        self.next
    }

    fn set_next(&mut self, next: *mut Self) {
        self.next = next;
    }

    fn prev(&self) -> *mut Self {
        self.prev
    }

    fn set_prev(&mut self, prev: *mut Self) {
        self.prev = prev;
    }

    // +------------------+
    // | pointer to slab  |
    // |    (8 bytes)     |
    // +------------------+ <- return value
    // |       data       |
    // | (size - 8 bytes) |
    // |                  |
    /// allocate a memory region whose size is 65504 bytes
    fn alloc(&mut self) -> *mut u8 {
        let ptr = &mut (self.buf[0]) as *mut u8;
        let ptr64 = ptr as *mut usize;

        // first 64 bits points the slab
        unsafe {
            *ptr64 = self as *mut Slab65512 as usize;
        }

        self.num = 1;

        &mut (self.buf[8]) as *mut u8
    }

    fn free(&mut self, _ptr: *mut u8) {
        self.num = 0;
    }

    fn is_full(&self) -> bool {
        true
    }

    fn is_empty(&self) -> bool {
        self.num == 0
    }

    fn init(&mut self) {
        self.next = null_mut();
        self.prev = null_mut();
        self.size = 65512;
        self.num = 0;
    }

    // fn print(&self) {
    //     driver::uart::puts("1");

    //     match unsafe { self.next.as_ref() } {
    //         Some(next) => {
    //             next.print();
    //         }
    //         None => {}
    //     }
    // }
}

// macro_rules! print_slabs {
//     ($s:literal, $slab_partial:ident, $slab_full:ident) => {
//         driver::uart::puts("\n");
//         driver::uart::puts($s);
//         driver::uart::puts("_partial:\n");
//         match unsafe { SLAB_ALLOC.$slab_partial.as_ref() } {
//             Some(slab) => {
//                 slab.print();
//             }
//             None => {}
//         }

//         driver::uart::puts("\n");
//         driver::uart::puts($s);
//         driver::uart::puts("_full:\n");
//         match unsafe { SLAB_ALLOC.$slab_full.as_ref() } {
//             Some(slab) => {
//                 slab.print();
//             }
//             None => {}
//         }
//     };
// }

// pub fn print_slabs() {
//     print_slabs!("slab16", slab16_partial, slab16_full);
//     print_slabs!("slab32", slab32_partial, slab32_full);
//     print_slabs!("slab64", slab64_partial, slab64_full);
//     print_slabs!("slab128", slab128_partial, slab128_full);
//     print_slabs!("slab256", slab256_partial, slab256_full);
//     print_slabs!("slab512", slab512_partial, slab512_full);
//     print_slabs!("slab1024", slab1024_partial, slab1024_full);
//     print_slabs!("slab2040", slab2040_partial, slab2040_full);
//     print_slabs!("slab4088", slab4088_partial, slab4088_full);
//     print_slabs!("slab8184", slab8184_partial, slab8184_full);
//     print_slabs!("slab16376", slab16376_partial, slab16376_full);
//     print_slabs!("slab32752", slab32752_partial, slab32752_full);
//     print_slabs!("slab65512", slab65512_partial, slab65512_full);
//     driver::uart::puts("\n");
// }
