// QEMU virt's sifive_test device: powers off the machine, setting QEMU's exit status
const TEST_BASE: usize = 0x10_0000;
const PASS: u32 = 0x5555;
const FAIL: u32 = 0x3333;

pub fn shutdown(code: u16) -> ! {
    let value = if code == 0 { PASS } else { (code as u32) << 16 | FAIL };
    unsafe { (TEST_BASE as *mut u32).write_volatile(value) };
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
