// kernel-wide sizes and addresses

// bytes in the kernel heap (a static array in .bss, see mm/heap.rs)
pub const KERNEL_HEAP_SIZE: usize = 1 << 20; // 1 MiB
