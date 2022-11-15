# memac: Memory allocator using slab and buddy allocators

## Usage

First of all, allocate 64KiB aligned memory regions for slab and buddy allocators.

```rust
#![feature(start)]

use memac;
use libc::{posix_memalign, c_void};

#[global_allocator]
static mut ALLOC: memac::Allocator<memac::budyy::Buddy32M> = memac::Allocator::new();

fn main() {
    println!("Hello, world!");
}

const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32MiB

#[start]
fn start(_argc: isize, _argv: *const *const u8) -> isize {
    // initialize memory
    unsafe {
        // allocate memory
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if posix_memalign(&mut ptr, memac::ALIGNMENT, HEAP_SIZE) != 0 {
            panic!("posix_memalign");
        }
        ALLOC.init(ptr as usize, HEAP_SIZE);
    }

    main();
    0
}
```

```toml
[dependencies]
libc = "0.2"
memac = "0.5"
```

`memac::Allocator<memac::budyy::Buddy32M>` means that
the slab allocator uses the buddy allocator to allocate slabs.
If a requested size is greater than (65512 - 8) bytes,
the buddy allocator is used to allocate memory.

`memac::Allocator<memac::pager::PageManager>` means that
the allocator uses the page manager to allocate slabs.
If a requested size is greater than (65512 - 8) bytes,
the page manager is used to allocate memory.
If a requested size is greater that 64K bytes,
the allocation will fail.
