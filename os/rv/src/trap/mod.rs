mod context;

pub use context::TrapContext;
use crate::timer;
use crate::task;
use core::arch::global_asm;

use crate::arch;

global_asm!(include_str!("trap.asm"));

unsafe extern "C" {
    fn __trap_entry();
}

const SCAUSE_INTERRUPT: usize = 1 << 63;
const EXC_BREAKPOINT: usize = 3;
const IRQ_SUPERVISOR_TIMER: usize = 5;

pub fn init() {
    arch::write_stvec(__trap_entry as usize);
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(ctx: &mut TrapContext) {
    let scause = arch::read_scause();
    let is_interrupt = scause & SCAUSE_INTERRUPT != 0;
    let code = scause & !SCAUSE_INTERRUPT;

    match (is_interrupt, code) {
        (false, EXC_BREAKPOINT) => {
            println!("breakpoint at {:#x}", ctx.sepc);
            ctx.sepc += instruction_len(ctx.sepc);
        },
        (true, IRQ_SUPERVISOR_TIMER) => {
            handle_timer_interrupt(ctx);
        }
        _ => panic!(
            "unhandled trap: interrupt={is_interrupt} code={code} sepc={:#x} stval={:#x}",
            ctx.sepc,
            arch::read_stval()
        ),
    }
}

// timer::on_tick, then (once tasks exist) preempt: task::yield_now
fn handle_timer_interrupt(ctx: &mut TrapContext) {
    timer::on_tick();
    task::on_tick(); // charges the running task; yields only if it's past the smallest Ready one
}

// low two bits 0b11 = full 4-byte instruction, anything else = 2-byte compressed
fn instruction_len(addr: usize) -> usize {
    let low = unsafe { (addr as *const u16).read_volatile() };
    if low & 0b11 == 0b11 { 4 } else { 2 }
}
