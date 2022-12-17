// Let 'h' be the depth of a complete binary tree,
// then the number of nodes is
// 2^(h+1) - 1 = (1 << (h + 1)) - 1
// .
//
// When h = 10,
// 2^10 * min_size
// is the maximum byte size of the buddy memory allocator.
//
// u: unused
// x: inner node
// L: used leaf node
// (number) indicates the index of a node
//       x(0)
//     /     \
//    x(1)    L(2)
//  /   \
// u(3) L(4) u(5) u(6)
//
// encoding rule
// 0b00: unused
// 0b01: inner node
// 0b10: used leaf
//
// above tree can be encoded as
// 01   01   10   00   10   00   00
// x(0) x(1) L(2) u(3) L(4) u(5) u(6)

use crate::{MemAlloc, SIZE_64K};

const TAG_UNUSED: u64 = 0;
const TAG_INNER: u64 = 1;
const TAG_USED_LEAF: u64 = 2;

pub struct BuddyAlloc<const DEPTH: usize, const NUM_NODES32: usize> {
    min_size: usize,
    start: usize,               // start address
    bitmap: [u64; NUM_NODES32], // succinct structure of the tree
}

// let num_nodes = (1 << (DEPTH_OF_TREE + 1)) - 1; // the number of nodes.
// (num_nodes >> 5) + 1 // #nodes / 32 + 1
//
// Because each node can be represented by 2 bits,
// `u64` can encode 32 nodes.
// So, at most (#nodes / 32 + 1) elements of `u64` are required to represent
// a tree of a buddy allocator.
const NODES_PAGE64K_MEM32M: usize = (((1 << (DEPTH_PAGE64K_MEM32M + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM64M: usize = (((1 << (DEPTH_PAGE64K_MEM64M + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM128M: usize = (((1 << (DEPTH_PAGE64K_MEM128M + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM256M: usize = (((1 << (DEPTH_PAGE64K_MEM256M + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM512M: usize = (((1 << (DEPTH_PAGE64K_MEM512M + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM1G: usize = (((1 << (DEPTH_PAGE64K_MEM1G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM2G: usize = (((1 << (DEPTH_PAGE64K_MEM2G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM4G: usize = (((1 << (DEPTH_PAGE64K_MEM4G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM8G: usize = (((1 << (DEPTH_PAGE64K_MEM8G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM16G: usize = (((1 << (DEPTH_PAGE64K_MEM16G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM32G: usize = (((1 << (DEPTH_PAGE64K_MEM32G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM64G: usize = (((1 << (DEPTH_PAGE64K_MEM64G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM128G: usize = (((1 << (DEPTH_PAGE64K_MEM128G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM256G: usize = (((1 << (DEPTH_PAGE64K_MEM256G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM512G: usize = (((1 << (DEPTH_PAGE64K_MEM512G + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM1T: usize = (((1 << (DEPTH_PAGE64K_MEM1T + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM2T: usize = (((1 << (DEPTH_PAGE64K_MEM2T + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM4T: usize = (((1 << (DEPTH_PAGE64K_MEM4T + 1)) - 1) >> 5) + 1;
const NODES_PAGE64K_MEM8T: usize = (((1 << (DEPTH_PAGE64K_MEM8T + 1)) - 1) >> 5) + 1;

const DEPTH_PAGE64K_MEM32M: usize = 9;
const DEPTH_PAGE64K_MEM64M: usize = 10;
const DEPTH_PAGE64K_MEM128M: usize = 11;
const DEPTH_PAGE64K_MEM256M: usize = 12;
const DEPTH_PAGE64K_MEM512M: usize = 13;
const DEPTH_PAGE64K_MEM1G: usize = 14;
const DEPTH_PAGE64K_MEM2G: usize = 15;
const DEPTH_PAGE64K_MEM4G: usize = 16;
const DEPTH_PAGE64K_MEM8G: usize = 17;
const DEPTH_PAGE64K_MEM16G: usize = 18;
const DEPTH_PAGE64K_MEM32G: usize = 19;
const DEPTH_PAGE64K_MEM64G: usize = 20;
const DEPTH_PAGE64K_MEM128G: usize = 21;
const DEPTH_PAGE64K_MEM256G: usize = 22;
const DEPTH_PAGE64K_MEM512G: usize = 23;
const DEPTH_PAGE64K_MEM1T: usize = 24;
const DEPTH_PAGE64K_MEM2T: usize = 25;
const DEPTH_PAGE64K_MEM4T: usize = 26;
const DEPTH_PAGE64K_MEM8T: usize = 27;

pub type Buddy32M = BuddyAlloc<DEPTH_PAGE64K_MEM32M, NODES_PAGE64K_MEM32M>;
pub type Buddy64M = BuddyAlloc<DEPTH_PAGE64K_MEM64M, NODES_PAGE64K_MEM64M>;
pub type Buddy128M = BuddyAlloc<DEPTH_PAGE64K_MEM128M, NODES_PAGE64K_MEM128M>;
pub type Buddy256M = BuddyAlloc<DEPTH_PAGE64K_MEM256M, NODES_PAGE64K_MEM256M>;
pub type Buddy512M = BuddyAlloc<DEPTH_PAGE64K_MEM512M, NODES_PAGE64K_MEM512M>;
pub type Buddy1G = BuddyAlloc<DEPTH_PAGE64K_MEM1G, NODES_PAGE64K_MEM1G>;
pub type Buddy2G = BuddyAlloc<DEPTH_PAGE64K_MEM2G, NODES_PAGE64K_MEM2G>;
pub type Buddy4G = BuddyAlloc<DEPTH_PAGE64K_MEM4G, NODES_PAGE64K_MEM4G>;
pub type Buddy8G = BuddyAlloc<DEPTH_PAGE64K_MEM8G, NODES_PAGE64K_MEM8G>;
pub type Buddy16G = BuddyAlloc<DEPTH_PAGE64K_MEM16G, NODES_PAGE64K_MEM16G>;
pub type Buddy32G = BuddyAlloc<DEPTH_PAGE64K_MEM32G, NODES_PAGE64K_MEM32G>;
pub type Buddy64G = BuddyAlloc<DEPTH_PAGE64K_MEM64G, NODES_PAGE64K_MEM64G>;
pub type Buddy128G = BuddyAlloc<DEPTH_PAGE64K_MEM128G, NODES_PAGE64K_MEM128G>;
pub type Buddy256G = BuddyAlloc<DEPTH_PAGE64K_MEM256G, NODES_PAGE64K_MEM256G>;
pub type Buddy512G = BuddyAlloc<DEPTH_PAGE64K_MEM512G, NODES_PAGE64K_MEM512G>;
pub type Buddy1T = BuddyAlloc<DEPTH_PAGE64K_MEM1T, NODES_PAGE64K_MEM1T>;
pub type Buddy2T = BuddyAlloc<DEPTH_PAGE64K_MEM2T, NODES_PAGE64K_MEM2T>;
pub type Buddy4T = BuddyAlloc<DEPTH_PAGE64K_MEM4T, NODES_PAGE64K_MEM4T>;
pub type Buddy8T = BuddyAlloc<DEPTH_PAGE64K_MEM8T, NODES_PAGE64K_MEM8T>;

enum Tag {
    Unused = TAG_UNUSED as isize,
    Inner = TAG_INNER as isize,
    UsedLeaf = TAG_USED_LEAF as isize,
}

impl<const DEPTH: usize, const NUM_NODES32: usize> BuddyAlloc<DEPTH, NUM_NODES32> {
    pub(crate) fn buddy_alloc(&mut self, size: usize) -> Option<*mut u8> {
        self.find_mem(size, (1 << DEPTH) * self.min_size, 0, 0)
    }

    pub(crate) fn buddy_free(&mut self, addr: *mut u8) {
        self.release_mem(addr as usize, (1 << DEPTH) * self.min_size, 0, 0)
    }

    fn get_tag(&self, idx: usize) -> Tag {
        let i = idx >> 5; // div by 32
        let j = idx & 0b11111;
        match (self.bitmap[i] >> (j * 2)) & 0b11 {
            TAG_UNUSED => Tag::Unused,
            TAG_INNER => Tag::Inner,
            TAG_USED_LEAF => Tag::UsedLeaf,
            _ => panic!("unknown tag"),
        }
    }

    fn set_tag(&mut self, idx: usize, tag: Tag) {
        let i = idx >> 5; // div by 32
        let j = idx & 0b11111;
        let mask = 0b11 << (j * 2);
        let val = self.bitmap[i] & !mask;
        self.bitmap[i] = val | ((tag as u64) << (j * 2));
    }

    fn get_idx(depth: usize, offset: usize) -> usize {
        if depth == 0 {
            0
        } else {
            (1 << depth) - 1 + offset
        }
    }

    fn find_mem(
        &mut self,
        req: usize,   // requested bytes
        bytes: usize, // total bytes of this block
        depth: usize,
        offset: usize, // offset of current node in the depth
    ) -> Option<*mut u8> {
        if req > bytes || depth > DEPTH {
            return None;
        }

        let idx = Self::get_idx(depth, offset);

        match self.get_tag(idx) {
            Tag::UsedLeaf => None,
            Tag::Unused => {
                let next_bytes = bytes >> 1;
                if next_bytes >= req && depth < DEPTH {
                    // divide
                    self.set_tag(idx, Tag::Inner);
                    self.find_mem(req, next_bytes, depth + 1, offset * 2)
                } else {
                    self.set_tag(idx, Tag::UsedLeaf);
                    let addr = self.start + bytes * offset;
                    let ptr = addr as *mut u8;
                    Some(ptr)
                }
            }
            Tag::Inner => match self.find_mem(req, bytes >> 1, depth + 1, offset * 2) {
                None => self.find_mem(req, bytes >> 1, depth + 1, offset * 2 + 1),
                ret => ret,
            },
        }
    }

    fn release_mem(&mut self, addr: usize, bytes: usize, depth: usize, offset: usize) {
        let idx = Self::get_idx(depth, offset);
        match self.get_tag(idx) {
            Tag::Unused => {
                panic!("freed unused memory");
            }
            Tag::UsedLeaf => {
                let target = self.start + bytes * offset;
                if target == addr {
                    self.set_tag(idx, Tag::Unused);
                } else {
                    panic!("freed invalid address");
                }
            }
            Tag::Inner => {
                let pivot = self.start + bytes * offset + (bytes >> 1);
                if addr < pivot {
                    self.release_mem(addr, bytes >> 1, depth + 1, offset * 2);
                } else {
                    self.release_mem(addr, bytes >> 1, depth + 1, offset * 2 + 1);
                }

                // combine buddy if both blocks are unused
                let left = Self::get_idx(depth + 1, offset * 2);
                let right = Self::get_idx(depth + 1, offset * 2 + 1);
                if let Tag::Unused = self.get_tag(left) {
                    if let Tag::Unused = self.get_tag(right) {
                        self.set_tag(idx, Tag::Unused);
                    }
                }
            }
        }
    }

    // pub fn print(&self) {
    //     for i in 0..(1 << (MAX_DEPTH + 1)) - 1 {
    //         uart::puts("idx = ");
    //         uart::decimal(i as u64);
    //         uart::puts(", tag = ");
    //         match self.get_tag(i) {
    //             Tag::Unused => uart::puts("unused\n"),
    //             Tag::Inner => uart::puts("inner\n"),
    //             Tag::UsedLeaf => uart::puts("used leaf\n"),
    //         }
    //     }
    // }
}

impl<const DEPTH: usize, const NUM_NODES32: usize> MemAlloc for BuddyAlloc<DEPTH, NUM_NODES32> {
    fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        self.buddy_alloc(size)
    }

    fn free(&mut self, addr: *mut u8) {
        self.buddy_free(addr)
    }

    fn new(start_addr: usize, size: usize) -> Self {
        assert_eq!(size, (1 << DEPTH) * SIZE_64K);

        Self {
            min_size: SIZE_64K,
            start: start_addr,
            bitmap: [0; NUM_NODES32],
        }
    }
}
