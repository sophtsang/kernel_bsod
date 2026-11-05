mod context;

pub use context::TaskContext;

use core::{arch::global_asm, cmp::{max, min}};
use crate::arch;
use crate::sync::{SpinLock, spinlock::SpinLockGuard};

global_asm!(include_str!("switch.asm"));

unsafe extern "C" {
    // saves the running task's registers into *current and resumes *next (switch.asm)
    // prologue stuff of calling conventions
    fn __switch(current: *mut TaskContext, next: *const TaskContext);
}

pub const MAX_TASKS: usize = 8;
pub const KERNEL_STACK_SIZE: usize = 16 * 1024;
pub const EPSILON: u64 = 10_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    Unused,
    Ready,
    Running,
    Exited,
}

pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub context: TaskContext,
    pub entry: Option<fn()>,
    pub exec_start: u64, // last time since this task was processed
    pub vruntime: u64
}

impl Task {
    // an Unused slot: what every entry of TaskManager::new()'s array starts as
    const fn empty() -> Self {
        Self {
            id: 0,
            state: TaskState::Unused,
            context: TaskContext::zeroed(),
            entry: None,
            exec_start: 0,
            vruntime: 0,
        }
    }
}

#[repr(C, align(16))]
pub struct KernelStack([u8; KERNEL_STACK_SIZE]);

static mut KERNEL_STACKS: [KernelStack; MAX_TASKS] =
    [const { KernelStack([0; KERNEL_STACK_SIZE]) }; MAX_TASKS];

impl KernelStack {
    // stacks grow down, so a new task's sp starts at the end
    pub fn top(&self) -> usize {
        self.0.as_ptr() as usize + KERNEL_STACK_SIZE // addr to start of stack + size
    }
}

pub struct TaskManager {
    tasks: [Task; MAX_TASKS], // replace with BTreeMap phase 2]
    next_id: usize,
    running_id: Option<usize>, // id of currently running task
    min_ready: Option<usize>, // idx within Ready (NOT RUNNING) tasks
    floor_vruntime: u64, // this is min(runtime Ready or Running), set new tasks' runtime to this
}

// once new() is implemented:
static TASK_MANAGER: SpinLock<TaskManager> = SpinLock::new(TaskManager::new());

impl TaskManager {
    pub const fn new() -> Self { // this is a circular buffer, fifo task store
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],
            next_id: 0,
            running_id: None,
            min_ready: None,
            floor_vruntime: 0
        }
    }

    // a task just became Ready: it's the new min_ready if it's below the current one
    fn add_ready(&mut self, idx: usize) {
        match self.min_ready {
            Some(m) if self.tasks[m].vruntime <= self.tasks[idx].vruntime => {}
            _ => self.min_ready = Some(idx),
        }
    }       


    // CFS, here it is guaranteed that the current task's SET has exceeded
    // min_vruntime -> context switch
    // at every tick, current task's SET + time_elapsed
    fn next_ready(&mut self) -> Option<usize> {
        // idx is the index of next Ready task // clear cache 
        if let Some(min_idx) = self.min_ready {
            self.min_ready = None;
            // this is the Ready task that has the smallest min_runtime after task[idx] is popped.
            for (curr_idx, curr_task) in self.tasks.iter().enumerate()
                .filter(|(_, task)| task.state == TaskState::Ready) {
                if curr_idx != min_idx {
                    let curr_runtime = curr_task.vruntime;
                    if let Some(idx) = self.min_ready && curr_runtime > self.tasks[idx].vruntime {
                        continue;
                    }
                    self.min_ready = Some(curr_idx);
                }
            }
            return Some(min_idx);
        }       
        None
    }
}

// claims an Unused slot, gives it a stack and a context that starts in task_start
pub fn spawn(entry: fn()) -> Option<usize> {
    // create a Task w/ entry and then add it to TaskManager's tasks, also update current?
    // also move sp   
    // top() gives the the top of the stack, and then we get the offset from next
    // slot that is used
    let mut tm = TASK_MANAGER.lock();

    for i in 0..MAX_TASKS {
        match tm.tasks[i].state {
            TaskState::Unused | TaskState::Exited => {
                let context = TaskContext::new_task(
                    task_start as usize, // ra: addr of task_start
                    unsafe { (*(&raw const KERNEL_STACKS[i])).top() }
                );
                let task = Task {
                    id: tm.next_id, state: TaskState::Ready, context: context, entry: Some(entry),
                    exec_start: arch::read_time(), // this is not accurate, cuz not guarantee run now
                    vruntime: tm.floor_vruntime
                };
                tm.tasks[i] = task;
                tm.next_id += 1;
                tm.add_ready(i);
                return Some(i)
            }
            _ => { continue; }
        }
    }
    None
}

// called by the trap handler on every timer interrupt: charge the running task for the time
// since it was switched in (or since the last tick), then move the floor forward
pub fn on_tick() {
    let mut tm = TASK_MANAGER.lock();
    let Some(cur) = tm.running_id else { return }; // still in boot code, nothing to charge

    let now = arch::read_time();
    let task = &mut tm.tasks[cur];
    task.vruntime += now - task.exec_start;
    task.exec_start = now;
    let running_vruntime = task.vruntime;

    // floor = max(old floor, min(running, smallest Ready)); with nothing Ready, just the running task
    let lowest = match tm.min_ready {
        Some(idx) => min(running_vruntime, tm.tasks[idx].vruntime),
        None => running_vruntime,
    };
    tm.floor_vruntime = max(tm.floor_vruntime, lowest);

    if let Some(min_idx) = tm.min_ready && tm.tasks[min_idx].vruntime + EPSILON < running_vruntime {
        drop(tm);
        yield_now();
    }
}

// switches from the boot code into the first Ready task; boot never resumes
pub fn run_first_task() -> ! {
    // for first task ever!! run only once i hope
    arch::disable_interrupts();
    let mut tm = TASK_MANAGER.lock();
    
    let next = tm.next_ready().expect("no task spawned in first task");

    tm.tasks[next].state = TaskState::Running;
    tm.tasks[next].exec_start = arch::read_time();
    tm.running_id = Some(next);
    // get the sp and ra of the new task
    let next_ctx = &raw const tm.tasks[next].context;
    let mut boot = TaskContext::zeroed();

    // drop the guard (from lock())
    drop(tm);
    unsafe { __switch(&raw mut boot, next_ctx) };
    unreachable!()
}

// current task gives up the CPU (Running -> Ready) and the next Ready one runs
pub fn yield_now() {
    let was_on = arch::disable_interrupts_saving(); // no prempting or interupting
    let mut tm = TASK_MANAGER.lock(); // get spinlock guard, when deref, get TaskManager
    
    if let Some(cur) = tm.running_id {
        let now = arch::read_time();
        let task = &mut tm.tasks[cur];
        task.state = TaskState::Ready;
        task.vruntime += now - task.exec_start;
        task.exec_start = now; // time to actually run the new task
        tm.add_ready(cur);
    }

    schedule(tm);
    arch::restore_interrupts(was_on);
}

// marks the current task Exited and switches away for good
// similar get next task logiuc to yield
pub fn exit_current() -> ! {
    // off for good on this task: it never resumes, so nothing restores them
    arch::disable_interrupts();
    let mut tm = TASK_MANAGER.lock();

    let cur = tm.running_id.expect("exit_current with no running task");
    tm.tasks[cur].state = TaskState::Exited;

    schedule(tm);
    println!("all tasks exited");
    crate::shutdown::shutdown(0)
}

// picks the next task and calls __switch. the TASK_MANAGER guard must be dropped before
// __switch (grab the two context pointers first), or the next task deadlocks on it
fn schedule(mut tm: SpinLockGuard<'_, TaskManager>) {
    let Some(next) = tm.next_ready() else { return }; // if there is new running task
    let prev = tm.running_id;

    tm.tasks[next].state = TaskState::Running;
    tm.tasks[next].exec_start = arch::read_time();
    tm.running_id = Some(next);
    if prev == Some(next) {
        return;
    }

    // get the sp and ra of the new task
    let next_ctx = &raw const tm.tasks[next].context; // ra, sp, data
    let prev_ctx = &raw mut tm.tasks[prev.expect("task not found?")].context;

    // drop the guard (from lock())
    drop(tm);
    unsafe { __switch(prev_ctx, next_ctx) }; // ra, sp of prev task in saved registers
    // __switch saves this function's own ra (a return address back into schedule) and sp into
    // prev_ctx, loads next's ra/sp/s0-s11, and `ret`s into next: wherever it last called
    // __switch, or task_start if it has never run. we resume on the line below only when
    // some later schedule() switches back to prev
}

pub fn current_id() -> Option<usize> {
    let tm = TASK_MANAGER.lock();
    // id (pid-like), not the slot index
    tm.running_id.map(|idx| tm.tasks[idx].id)
}

// first code a new task runs (its context's ra): enable interrupts, call entry, exit
extern "C" fn task_start() -> ! {
    // reached via schedule's __switch: ra held task_start's address, sp the new stack.
    // read this task's entry fn pointer from its slot (running_id = this task), then call it
    let tm = TASK_MANAGER.lock();
    let idx = tm.running_id.expect("task_start with no running task");
    let entry = tm.tasks[idx].entry.expect("task has no entry");
    drop(tm);

    arch::enable_interrupts();

    entry(); // the task's actual code
    exit_current()
}

#[cfg(feature = "test")]
pub fn vruntime_of(slot: usize) -> u64 {
    TASK_MANAGER.lock().tasks[slot].vruntime
}
