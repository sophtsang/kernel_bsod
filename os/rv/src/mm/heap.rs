use alloc_algos::Memory;
use crate::sync::SpinLock;
use crate::config::KERNEL_HEAP_SIZE;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};

struct KernelHeap(SpinLock<Memory>);

#[global_allocator]
static HEAP: KernelHeap = KernelHeap(SpinLock::new(Memory::new()));

// the heap's memory: a page-aligned array in .bss
#[repr(C, align(4096))]
struct HeapSpace([u8; KERNEL_HEAP_SIZE]);

static mut HEAP_SPACE: HeapSpace = HeapSpace([0; KERNEL_HEAP_SIZE]);

// call once, after clear_bss (it zeroes HEAP_SPACE) and before anything allocates
pub fn init() {
    let mut mem = HEAP.0.lock();
    unsafe { mem.init(&raw mut HEAP_SPACE as usize, KERNEL_HEAP_SIZE) };
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut mem = self.0.lock();
        match mem.allocate(layout) {
            Ok(addr) => addr.as_ptr(),
            Err(()) => ptr::null_mut(), // out of memory: the caller turns null into a panic
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut mem = self.0.lock();
        // GlobalAlloc guarantees ptr came from alloc, so it's never null
        unsafe { mem.deallocate(NonNull::new_unchecked(ptr), layout) };
    }
}
