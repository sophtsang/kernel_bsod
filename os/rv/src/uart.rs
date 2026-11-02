// NS16550A on QEMU virt
use core::fmt;

use crate::sync::SpinLock;

const UART_BASE: usize = 0x1000_0000;
const THR: usize = 0; // transmit holding register
const LSR: usize = 5; // line status register
const LSR_THR_EMPTY: u8 = 1 << 5;

pub struct Uart;

impl Uart {
    pub fn putc(&mut self, c: u8) {
        unsafe {
            while ((UART_BASE + LSR) as *const u8).read_volatile() & LSR_THR_EMPTY == 0 {}
            ((UART_BASE + THR) as *mut u8).write_volatile(c);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.putc(b);
        }
        Ok(())
    }
}

static UART: SpinLock<Uart> = SpinLock::new(Uart);

pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    UART.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::uart::_print(format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::print!("{}\n", format_args!($($arg)*)) };
}
