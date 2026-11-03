use core::mem::{offset_of, size_of};

// what switch.asm saves/loads; field offsets must match the asm exactly
#[repr(C)]
pub struct TaskContext {
    pub ra: usize,       // 0
    pub sp: usize,       // 8
    pub s: [usize; 12],  // 16 + i*8
}

const _: () = {
    assert!(offset_of!(TaskContext, ra) == 0);
    assert!(offset_of!(TaskContext, sp) == 8);
    assert!(offset_of!(TaskContext, s) == 16);
    assert!(size_of::<TaskContext>() == 14 * 8);
};

impl TaskContext {
    // all zeros: the slot a task's registers get saved into the first time it switches away
    pub const fn zeroed() -> Self {
        Self { ra: 0, sp: 0, s: [0; 12] }
    }

    // context for a task that has never run: __switch "returns" into `entry` on `stack_top`
    pub fn new_task(entry: usize, stack_top: usize) -> Self {
        Self { ra: entry, sp: stack_top }
    } 
}
