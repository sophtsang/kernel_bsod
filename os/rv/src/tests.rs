// in-kernel test runner: `cargo ktest` boots QEMU, runs every test, and exits 0.
// a failed assert panics, and the panic handler exits QEMU with status 1
use crate::shutdown::shutdown;
use crate::sync::spinlock;

const TESTS: &[(&str, fn())] = &[
    ("bss lies between sbss and ebss and is zeroed", bss_is_zeroed),
    ("data keeps its initial value", data_is_initialized),
    ("spinlock guard gives access to the data", spinlock::tests::guard_gives_access),
    ("spinlock unlocks when the guard drops", spinlock::tests::unlocks_on_drop),
    ("ebreak traps and execution continues after it", ebreak_returns),
    ("registers survive a trap", registers_survive_trap),
];

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
