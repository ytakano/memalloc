# memac: Memory allocator using slab and buddy allocators

## Usage

First of all, allocate 64KiB aligned memory regions for slab and buddy allocators.

```rust
#![feature(start)]

use memac;
use libc::{posix_memalign, c_void};

#[global_allocator]
static mut ALLOC: memac::Allocator = memac::Allocator::new();

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
        if posix_memalign(&mut ptr, memalloc::ALIGNMENT, HEAP_SIZE) != 0 {
            panic!("posix_memalign");
        }
        ALLOC.init(ptr as usize);
    }

    main();
    0
}
```

```toml
[dependencies]
libc = "0.2"
memac = { version="0.4", features=["buddy_32m"], default-features=false }
```

buddy_32m indicates that the buddy allocator's memory size is 32MiB.
If you want to change the size, see Cargo.toml.

The slab allocator uses the buddy allocator to allocate slabs.
