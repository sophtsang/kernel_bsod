mod heap;

// memory setup, in order: heap (later: frames, kernel page table, paging on)
pub fn init() {
    heap::init();
}
