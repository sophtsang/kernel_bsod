mod context;

pub use context::TaskContext;

use core::arch::global_asm;

global_asm!(include_str!("switch.asm"));

unsafe extern "C" {
    // saves the running task's registers into *current and resumes *next (switch.asm)
    fn __switch(current: *mut TaskContext, next: *const TaskContext);
}

pub const MAX_TASKS: usize = 8;
pub const KERNEL_STACK_SIZE: usize = 16 * 1024;

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
    pub vruntime: usize
}

#[repr(C, align(16))]
pub struct KernelStack([u8; KERNEL_STACK_SIZE]);

static mut KERNEL_STACKS: [KernelStack; MAX_TASKS] =
    [const { KernelStack([0; KERNEL_STACK_SIZE]) }; MAX_TASKS];

impl KernelStack {
    // stacks grow down, so a new task's sp starts at the end
    pub fn top(&self) -> usize {
        self.0.as_ptr() as usize + KERNEL_STACK_SIZE; // addr to start of stack + size
    }
}

pub struct TaskManager {
    tasks: [Task; MAX_TASKS], // replace with BTreeMap phase 2]
    next_id: usize,
    min_vruntime: usize
}

// once new() is implemented:
static TASK_MANAGER: SpinLock<TaskManager> = SpinLock::new(TaskManager::new());

impl TaskManager {
    pub const fn new() -> Self { // this is a circular buffer, fifo task store
        Self {
            tasks: [Task, MAX_TASKS],
            next_id: 0,
            min_vruntime: 0
        }
    }

    // round-robin: first Ready task after `current`, wrapping around
    fn next_ready(&self) -> Option<usize> {
        let idx = self.current.map_or(0, |c| c + 1);
        for offset in 0..MAX_TASKS {
            let task = &self.tasks[(idx + offset) % MAX_TASKS];
            if task.state == TaskState::Ready {
                return Some((idx + offset) & MAX_TASKS);
            }
        }
        None
    }
}

// pub struct Task {
//     pub id: usize,
//     pub state: TaskState,
//     pub context: TaskContext,
//     pub entry: Option<fn()>,
// // 
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
                    task_start as usize,
                    unsafe { *(&raw const KERNEL_STACKS[i]).top() }
                );
                let task = Task {
                    id: tm.next_id, state: TaskState::Ready, context: context, entry: Some(entry),
                    // max(old = 0, min_vruntime) = min_vruntime
                    vruntime: tm.min_vruntime
                };
                tm.tasks[i] = task;
                tm.next_id += 1;
                return Some(i)
            }
            _ => { continue; }
        }
    }
    None
}

// switches from the boot code into the first Ready task; boot never resumes
pub fn run_first_task() -> ! {
    todo!()
}

// current task gives up the CPU (Running -> Ready) and the next Ready one runs
pub fn yield_now() {
    todo!()
}

// marks the current task Exited and switches away for good
pub fn exit_current() -> ! {
    todo!()
}

// picks the next task and calls __switch. the TASK_MANAGER guard must be dropped before
// __switch (grab the two context pointers first), or the next task deadlocks on it
fn schedule() {
    todo!()
}

pub fn current_id() -> usize {
    todo!()
}

// first code a new task runs (its context's ra): enable interrupts, call entry, exit
extern "C" fn task_start() -> ! {
    // this is the first task that a new task runs, so its ra will be the location
    // in the stack where the task actually is
    todo!()
}
