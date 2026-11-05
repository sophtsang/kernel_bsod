// phase 1 demo: two tasks that never yield, so they only interleave because the timer
// preempts them; a third yields voluntarily. the last one to exit powers the machine off
use crate::{task, timer};

const ROUNDS: usize = 4;
const WORK_MS: u64 = 30;

// busy-waits WORK_MS between prints; never calls yield_now
fn spin(name: &str) {
    for i in 0..ROUNDS {
        let until = timer::now_ms() + WORK_MS;
        while timer::now_ms() < until {}
        println!("{name}{i} at {} ms", timer::now_ms());
    }
}

fn spinner_a() {
    spin("A")
}

fn spinner_b() {
    spin("B")
}

fn polite() {
    for i in 0..ROUNDS {
        println!("polite{i} (task {:?}) yields", task::current_id());
        task::yield_now();
    }
}

pub fn spawn_all() {
    for entry in [spinner_a as fn(), spinner_b, polite] {
        task::spawn(entry).expect("no free task slot");
    }
}
