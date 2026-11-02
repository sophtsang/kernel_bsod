#![no_std]
#![no_main]

#[macro_use]
mod uart;
mod shutdown;
mod sync;

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("entry.asm"));

unsafe extern "C" {
    static sbss: u8;
    static ebss: u8;
}

fn clear_bss() {
    let start = &raw const sbss as usize;
    let end = &raw const ebss as usize;
    unsafe { core::ptr::write_bytes(start as *mut u8, 0, end - start) };
}

#[unsafe(no_mangle)]
extern "C" fn rust_main(hart_id: usize, dtb: usize) -> ! {
    clear_bss();
    println!("kernel_bsod rv {} (hart {hart_id}, dtb {dtb:#x})", env!("CARGO_PKG_VERSION"));
    shutdown::shutdown(0)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(loc) => println!("panic at {}:{}: {}", loc.file(), loc.line(), info.message()),
        None => println!("panic: {}", info.message()),
    }
    shutdown::shutdown(1)
}
