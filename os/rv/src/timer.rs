use crate::sbi;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::arch;

// QEMU virt's `time` frequency; the real value is timebase-frequency in the device tree
pub const TIMEBASE_HZ: u64 = 10_000_000;
pub const TICKS_PER_SEC: u64 = 100;

static TICKS: AtomicU64 = AtomicU64::new(0);
static NEXT_DEADLINE: AtomicU64 = AtomicU64::new(0);

const TICK_CYCLES: u64 = TIMEBASE_HZ / TICKS_PER_SEC;

// schedules the first tick and enables the supervisor timer interrupt (not sstatus.SIE)
pub fn init() {
    let first = arch::read_time() + TICK_CYCLES;
    NEXT_DEADLINE.store(first, Ordering::Relaxed);
    sbi::set_timer(first);
    arch::enable_timer_interrupt();
}

// next deadline = previous deadline + one period (not now + one period), so handler latency
// doesn't accumulate. if we fell more than a period behind, skip to the first future deadline.
// returns how many periods passed; also clears the pending interrupt
pub fn set_next_tick() -> u64 {
    let now = arch::read_time();
    let mut next = NEXT_DEADLINE.load(Ordering::Relaxed);
    let mut periods = 0;
    if now >= next {
        periods = (now - next) / TICK_CYCLES + 1;
        next += periods * TICK_CYCLES;
    }
    NEXT_DEADLINE.store(next, Ordering::Relaxed);
    sbi::set_timer(next);
    periods
}

// called by trap_handler on a timer interrupt: set_next_tick first, then count
// every elapsed period, so ticks() stays in step with real time even after a missed one
pub fn on_tick() {
    let periods = set_next_tick();
    TICKS.fetch_add(periods, Ordering::Relaxed);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn now_ms() -> u64 {
    arch::read_time() / (TIMEBASE_HZ / 1000)
}
