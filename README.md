# memalloc: Memory allocator using slab and buddy allocators

## Usage

First of all, allocate 64KiB aligned memory regions for slab and buddy allocators.

```rust
#![feature(start)]

use memalloc;
use libc::{posix_memalign, c_void};

#[global_allocator]
static mut ALLOC: memalloc::Allocator = memalloc::Allocator::new();

fn main() {
    println!("Hello, world!");
}

const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32MiB

#[start]
fn start(_argc: isize, _argv: *const *const u8) -> isize {
    // initialize memory
    unsafe {
        // allocate memory for the buddy allocator
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if posix_memalign(&mut ptr, memalloc::ALIGNMENT, HEAP_SIZE) != 0 {
            panic!("posix_memalign");
        }
        ALLOC.init_buddy(ptr as usize);

        // allocate memory for the slab allocator
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if posix_memalign(&mut ptr, memalloc::ALIGNMENT, HEAP_SIZE) != 0 {
            panic!("posix_memalign");
        }
        ALLOC.init_slab(ptr as usize, HEAP_SIZE);
    }

    main();
    0
}
```

```toml
[dependencies]
libc = "0.2.85"
memalloc = { features=["buddy_32m"] }
```

buddy_32m indicates that the buddy allocator's memory size is 32MiB.
If you want to change the size, see Cargo.toml.

Slab allocator's size can be determined by the init_slab function.