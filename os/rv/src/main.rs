#![no_std]
#![no_main]

extern crate alloc;

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
mod config;
mod mm;
#[cfg(not(feature = "test"))]
mod demo;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

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
    mm::init(); // after clear_bss: the heap lives in .bss
    trap::init();
    println!("kernel_bsod rv {} (hart {hart_id}, dtb {dtb:#x})", env!("CARGO_PKG_VERSION"));

    #[cfg(feature = "test")]
    task::spawn(tests::runner).expect("no free task slot");
    #[cfg(not(feature = "test"))]
    demo::spawn_all();

    // interrupts stay off here: task_start turns them on inside each task
    timer::init();
    task::run_first_task()
}

static PANICKING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // no timer interrupt (and so no task switch) mid-panic
    arch::disable_interrupts();
    // a panic inside the panic handler (e.g. a bad Display impl): don't recurse, just stop
    if PANICKING.swap(true, Ordering::Relaxed) {
        shutdown::shutdown(2)
    }
    // unlocked: the panic may have happened while UART's lock was held
    match info.location() {
        Some(loc) => uart::_print_unlocked(format_args!(
            "panic at {}:{}: {}\n", loc.file(), loc.line(), info.message()
        )),
        None => uart::_print_unlocked(format_args!("panic: {}\n", info.message())),
    }
    shutdown::shutdown(1)
}
