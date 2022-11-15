use crate::{MemAlloc, SIZE_64K};

/// 64 * 64 * 64 pages = 64 * 64 * 64 * 64KiB = 16GiB
pub struct PageManager {
    start: usize,
    end: usize,
    vacancy_books: u64,
    vacancy_pages: [u64; 64],
    book: [Book; 64],
}

#[derive(Copy, Clone)]
pub struct Book {
    pages: [u64; 64],
}

impl PageManager {
    // pub fn print(&self) {
    //     uart::puts("start = 0x");
    //     uart::hex(self.start as u64);
    //     uart::puts("\nend = 0x");
    //     uart::hex(self.end as u64);
    //     uart::puts("\nvacancy_books = 0x");
    //     uart::hex(self.vacancy_books);
    //     uart::puts("\nvacancy_pages:");
    //     let mut i = 0;
    //     for p in self.vacancy_pages.iter() {
    //         uart::puts("\n  ");
    //         uart::decimal(i);
    //         uart::puts(": 0x");
    //         uart::hex(*p);
    //         i += 1;
    //     }
    //     uart::puts("\n");
    // }

    pub fn page_alloc(&mut self) -> Option<*mut u8> {
        if self.vacancy_books == !0 {
            return None;
        }

        let idx1 = (!self.vacancy_books).leading_zeros() as usize;
        let idx2 = (!self.vacancy_pages[idx1]).leading_zeros() as usize;
        let idx3 = (!self.book[idx1].pages[idx2]).leading_zeros() as usize;

        let addr =
            64 * 1024 * 64 * 64 * idx1 + 64 * 1024 * 64 * idx2 + 64 * 1024 * idx3 + self.start;

        if addr >= self.end {
            return None;
        }

        self.book[idx1].pages[idx2] |= 1 << (63 - idx3);
        if self.book[idx1].pages[idx2] == !0 {
            self.vacancy_pages[idx1] |= 1 << (63 - idx2);
            if self.vacancy_pages[idx1] == !0 {
                self.vacancy_books |= 1 << (63 - idx1);
            }
        }

        Some(addr as _)
    }

    pub fn page_free(&mut self, addr: *mut u8) {
        let addr = addr as usize;
        if addr & 0xFFFF != 0 || addr >= self.end || addr < self.start {
            panic!("invalid address");
        }

        let idx1 = ((addr - self.start) >> 28) & 0b111111;
        let idx2 = (addr >> 22) & 0b111111;
        let idx3 = (addr >> 16) & 0b111111;

        self.book[idx1].pages[idx2] &= !(1 << (63 - idx3));
        self.vacancy_pages[idx1] &= !(1 << (63 - idx2));
        self.vacancy_books &= !(1 << (63 - idx1));
    }
}

impl MemAlloc for PageManager {
    fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        if size > SIZE_64K {
            None
        } else {
            self.page_alloc()
        }
    }

    fn free(&mut self, addr: *mut u8) {
        self.page_free(addr)
    }

    fn new(start_addr: usize, size: usize) -> Self {
        assert_eq!(size % SIZE_64K, 0);

        PageManager {
            start: start_addr,
            end: start_addr + size,
            vacancy_books: 0,
            vacancy_pages: [0; 64],
            book: [Book { pages: [0; 64] }; 64],
        }
    }
}
