#![no_std]
#![no_main]

#[macro_use]
mod uart;
mod shutdown;
mod sync;
#[cfg(feature = "test")]
mod tests;
mod arch;
mod trap;
mod sbi;
mod timer;
mod task;

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
    trap::init();
    timer::init();
    arch::enable_interrupts();
    println!("kernel_bsod rv {} (hart {hart_id}, dtb {dtb:#x})", env!("CARGO_PKG_VERSION"));

    #[cfg(feature = "test")]
    tests::run();

    #[cfg(not(feature = "test"))]
    tick_demo()
}

// phase 1 checkpoint: wait for 3 s worth of ticks, printing once a second, then power off.
// elapsed wall time vs ticks shows how far the timer drifts
#[cfg(not(feature = "test"))]
fn tick_demo() -> ! {
    let start = timer::now_ms();
    let mut seconds = 0;
    while seconds < 3 {
        arch::wait_for_interrupt();
        let ticks = timer::ticks();
        if ticks / timer::TICKS_PER_SEC > seconds {
            seconds = ticks / timer::TICKS_PER_SEC;
            println!("{ticks} ticks ({seconds} s), {} ms elapsed", timer::now_ms() - start);
        }
    }
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
