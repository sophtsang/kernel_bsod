use core::arch::asm;

const SSTATUS_SIE: usize = 1 << 1;

pub fn read_sstatus() -> usize {
    let value: usize;
    unsafe { asm!("csrr {}, sstatus", out(reg) value) };
    value
}

pub fn read_scause() -> usize {
    let value: usize;
    unsafe { asm!("csrr {}, scause", out(reg) value) };
    value
}

pub fn read_stval() -> usize {
    let value: usize;
    unsafe { asm!("csrr {}, stval", out(reg) value) };
    value
}

pub fn write_stvec(addr: usize) {
    // stvec is a control and status register: trap vector stuff
    // this is: csrw stvec, addr
    // csr: aux reg in cpu that is used for reading status and
    // changing configs
    unsafe { asm!("csrw stvec, {}", in(reg) addr) };
}

pub fn enable_interrupts() {
    // csrs sets specific bits in csr
    unsafe { asm!("csrs sstatus, {}", in(reg) SSTATUS_SIE) };
}

pub fn disable_interrupts() {
    // clear bits in SSTATUS_SIE to 0, so disable interrupt, if was 1
    unsafe { asm!("csrc sstatus, {}", in(reg) SSTATUS_SIE) };
}

pub fn restore_interrupts(was_on: bool) {
    if was_on {
        enable_interrupts()
    }
}

pub fn disable_interrupts_saving() -> bool {
    // read value, change it, save interruupts tate
    let old: usize;
    unsafe { asm!("csrrc {}, sstatus, {}", out(reg) old, in(reg) SSTATUS_SIE) };
    // & is getting only the bits in old that is SSTATUS_SIE, and then
    // check if interrupt had been on prior to update
    old & SSTATUS_SIE != 0
}

pub fn wait_for_interrupt() {
    unsafe { asm!("wfi", options(nomem, nostack)) };
}

const SIE_STIE: usize = 1 << 5;

// cycles of the platform timer; 10 MHz on QEMU virt (timebase-frequency in the device tree)
pub fn read_time() -> u64 {
    let value: u64;
    unsafe { asm!("csrr {}, time", out(reg) value, options(nomem, nostack)) };
    value
}

pub fn enable_timer_interrupt() {
    unsafe { asm!("csrs sie, {}", in(reg) SIE_STIE) };
}

pub fn disable_timer_interrupt() {
    unsafe { asm!("csrc sie, {}", in(reg) SIE_STIE) };
}
