// in-kernel test runner: `cargo ktest` boots QEMU, runs every test, and exits 0.
// a failed assert panics, and the panic handler exits QEMU with status 1.
// the runner is itself a task (rust_main spawns it), so tests can spawn tasks and yield
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::shutdown::shutdown;
use crate::sync::{spinlock, SpinLock};
use crate::{arch, task, timer};

const TESTS: &[(&str, fn())] = &[
    ("bss lies between sbss and ebss and is zeroed", bss_is_zeroed),
    ("data keeps its initial value", data_is_initialized),
    ("spinlock guard gives access to the data", spinlock::tests::guard_gives_access),
    ("spinlock unlocks when the guard drops", spinlock::tests::unlocks_on_drop),
    ("ebreak traps and execution continues after it", ebreak_returns),
    ("registers survive a trap", registers_survive_trap),
    ("spinlock saves and restores interrupts", spinlock_restores_interrupts),
    ("timer ticks advance during a busy wait", timer_ticks_advance),
    ("yield_now with nothing else ready keeps running", lone_yield_returns),
    ("yield_now lets a spawned task run to completion", yield_runs_spawned_task),
    ("non-yielding tasks are both preempted and make progress", preempted_tasks_both_progress),
];

// entry for the runner task
pub fn runner() {
    run()
}

pub fn run() -> ! {
    println!("running {} tests", TESTS.len());
    for (name, test) in TESTS {
        print!("  {name} ... ");
        test();
        println!("ok");
    }
    println!("test result: ok. {} passed", TESTS.len());
    shutdown(0)
}

unsafe extern "C" {
    static sbss: u8;
    static ebss: u8;
}

static mut ZEROED: [u64; 4] = [0; 4];
static mut INITIALIZED: u64 = 0xdead_beef;

fn bss_is_zeroed() {
    let (start, end) = (&raw const sbss as usize, &raw const ebss as usize);
    let addr = &raw const ZEROED as usize;
    assert!(start <= addr && addr < end, "ZEROED at {addr:#x} is outside .bss {start:#x}..{end:#x}");
    let values = unsafe { (&raw const ZEROED).read_volatile() };
    assert_eq!(values, [0; 4]);
}

fn data_is_initialized() {
    let value = unsafe { (&raw const INITIALIZED).read_volatile() };
    assert_eq!(value, 0xdead_beef);
}

fn ebreak_returns() {
    unsafe { core::arch::asm!("ebreak") };
}

// caller-saved registers are exactly the ones the trap handler's Rust code clobbers,
// so they only survive if trap.asm saves and restores them
fn registers_survive_trap() {
    let (mut a, mut b, mut c) = (0x1111_usize, 0x2222_usize, 0x3333_usize);
    unsafe { core::arch::asm!("ebreak", inout("t0") a, inout("a5") b, inout("t6") c) };
    assert_eq!((a, b, c), (0x1111, 0x2222, 0x3333));
}

fn sie_on() -> bool {
    arch::read_sstatus() & (1 << 1) != 0
}

// the runner task starts with interrupts on (task_start enables them)
fn spinlock_restores_interrupts() {
    let (outer, inner) = (SpinLock::new(()), SpinLock::new(()));
    assert!(sie_on());

    let a = outer.lock();
    assert!(!sie_on(), "interrupts on while holding a lock");
    let b = inner.lock();
    drop(b);
    assert!(!sie_on(), "inner drop re-enabled interrupts while outer still held");
    drop(a);
    assert!(sie_on(), "outer drop didn't restore interrupts");

    arch::disable_interrupts();
    drop(outer.lock());
    assert!(!sie_on(), "lock re-enabled interrupts that were off before it");
    arch::enable_interrupts();
}

fn timer_ticks_advance() {
    let start = timer::ticks();
    let until = timer::now_ms() + 50;
    while timer::now_ms() < until {}
    // 50 ms is 5 ticks; allow slack for preemption and delivery latency
    assert!(timer::ticks() >= start + 3, "only {} ticks in 50 ms", timer::ticks() - start);
}

// no other task Ready: add_ready makes the yielding task the min, so schedule keeps it
fn lone_yield_returns() {
    let me = task::current_id();
    task::yield_now();
    assert_eq!(task::current_id(), me);
}

static RAN: AtomicBool = AtomicBool::new(false);

fn sets_ran() {
    RAN.store(true, Ordering::Relaxed);
}

fn yield_runs_spawned_task() {
    RAN.store(false, Ordering::Relaxed);
    task::spawn(sets_ran).expect("no free task slot");
    let until = timer::now_ms() + 1000;
    while !RAN.load(Ordering::Relaxed) {
        assert!(timer::now_ms() < until, "spawned task never ran");
        task::yield_now();
    }
}

static STOP: AtomicBool = AtomicBool::new(false);
static DONE: AtomicUsize = AtomicUsize::new(0);
static COUNT_A: AtomicU64 = AtomicU64::new(0);
static COUNT_B: AtomicU64 = AtomicU64::new(0);

// never yields: only the timer can take the CPU away
fn count_until_stopped(counter: &AtomicU64) {
    while !STOP.load(Ordering::Relaxed) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
    DONE.fetch_add(1, Ordering::Relaxed);
}

fn counter_a() {
    count_until_stopped(&COUNT_A)
}

fn counter_b() {
    count_until_stopped(&COUNT_B)
}

fn preempted_tasks_both_progress() {
    STOP.store(false, Ordering::Relaxed);
    DONE.store(0, Ordering::Relaxed);
    let a = task::spawn(counter_a).expect("no free task slot");
    let b = task::spawn(counter_b).expect("no free task slot");

    // the runner busy-waits too (no yield), so all three are only ever switched by preemption
    let until = timer::now_ms() + 300;
    while timer::now_ms() < until {}
    STOP.store(true, Ordering::Relaxed);
    while DONE.load(Ordering::Relaxed) < 2 {
        task::yield_now();
    }

    let (ca, cb) = (COUNT_A.load(Ordering::Relaxed), COUNT_B.load(Ordering::Relaxed));
    assert!(ca > 0 && cb > 0, "a task never ran: a={ca} b={cb}");

    // same work, same start: CFS keeps their CPU time within about a tick or two
    let (va, vb) = (task::vruntime_of(a), task::vruntime_of(b));
    let gap = va.abs_diff(vb);
    assert!(gap <= 2 * timer::TIMEBASE_HZ / timer::TICKS_PER_SEC, "vruntime a={va} b={vb}, gap {gap}");
}

// not in TESTS: panicking ends the run. checked by hand (panic while UART's lock is held still
// prints and exits 1); see uart::_print_unlocked
