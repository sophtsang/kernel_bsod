use core::mem::{offset_of, size_of};

// what trap.asm saves on every trap; field offsets must match the asm exactly
#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],  // x[i] at i*8; x[2] is sp from before the trap
    pub sstatus: usize,  // 256
    pub sepc: usize,     // 264
}

// compile-time check that this struct and trap.asm agree
const _: () = {
    assert!(offset_of!(TrapContext, sstatus) == 32 * 8);
    assert!(offset_of!(TrapContext, sepc) == 33 * 8);
    assert!(size_of::<TrapContext>() == 34 * 8);
};
