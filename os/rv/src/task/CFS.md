## Implementation Documentation for Kernel::BSOD's Scheduler: Completely Fair Scheduler

### From CS4410 and Linux Documentation:
"80% of CFS's design can be summed up in a single sentence: CFS basically models an "ideal, precise multi-tasking CPU" on real hardware". 

_Virtual runtime_: of a task specifies when its next timeslice would start execution on the ideal multi-tasking CPU. In practice, the virtual runtime of a task is its actual runtime normalized to the total number of running tasks. 

In CFS, virtual runtime is expressed by the per-task ```p->se.vruntime``` (nanosec-unit) value.

Completely Fair Scheduler defines the virtual runtime or "Spent Execution Time" (SET) to be the amount of time that a process (not job) has been executing:
1. Scheduler selects process with the lowest SET
    - Always tries to run the task with the smallest ```p->se.vruntime``` value (the task which has been executed least so far). 
2. Process runs until its SET is no longer the smallest among runnable processes
    - CFS tries to split up CPU time between runnable tasks as close to "ideal multitasking hardware" as possible.
3. Repeat

### RunQueue Data Structure: RBTree
CFS uses a time-ordered rbtree to build a "timeline" of future task execution. CFS also maintains a ```rq->cfs.min_vruntime``` value, which is used to place newly activated entities (processes) on the left side of the tree as much as possible.

In other words, if a process is new or it sleeps and wakes up, then its new SET is the maximum of its old SET and the minimum of the SETs of the processes on the run queue. 
- New or sleeping processes have SET = max(old SET, min(SET of current tasks)).

Total number of running tasks in the runqueue is accounted through the ```rq->cfs.load``` value, which is the sum of the weights (SETs) of the tasks queued on the runqueue. 

The runqueue is a time-ordered rbtree, where all runnable tasks are sorted by their SET. In ```next_ready```, CFS will pick the "leftmost" READY task from this tree. As the system progreses forwards, the executed tasks are put into the tree more to the right, as their SET increases. This gives a change for every task to become the "leftmost task". 

A task continues running until a different task becomes the "leftmost task" in the tree. 

```
pub struct Task {
    ...,
    vruntime: some time type
}
```

```
impl TaskManager {
    pub const fn new() -> Self {
        Self {
            min_vruntime: some time type, // the smallest vruntime among the running task and the Ready tasks
            tasks: [Task; usize] // before i impl memory allocator
            tasks: alloc::collections::BTreeMap // change to rbtree if have much more tasks
        } // the plan is that we pop the currently running task out of tasks, and then when its set is updated, we check with the new head of the binary tree / also check with new spawned process
        // this occurs at every tick (so quantum)
    }
```

```
    fn next_ready(&self) -> Option<usize> {
        get the leftmost task in the tree
        if it is the same as the currently
        scheduled, don't switch
        if different -> context switch
    }
}
```

```
pub fn spawn(entry: fn()) -> usize {
    inserting a new Task node to the rbtree
    its SET is min SET of current tasks?
    also may need wakeup_preempt: which is preemptively switching away from currently running task
}
```

```
// switches from the boot code into the first Ready task; boot never resumes
pub fn run_first_task() -> ! {
    todo!()
}
```
```
// current task gives up the CPU (Running -> Ready) and the next Ready one runs
pub fn yield_now() {
    // function yields the CPU by moving the currently running task's position back into the rbtree (runqueue), so that other runnable tasks get scheduled first
}
```

```
// marks the current task Exited and switches away for good
pub fn exit_current() -> ! {
    remove the current task from the rbtree permanently
}
```

```
// picks the next task and calls __switch. the TASK_MANAGER guard must be dropped before
// __switch (grab the two context pointers first), or the next task deadlocks on it
fn schedule() {
    todo!()
}
```

```
pub fn current_id() -> usize {
    todo!()
}
```

```
// first code a new task runs (its context's ra): enable interrupts, call entry, exit
extern "C" fn task_start() -> ! {
    todo!()
}
```


we spawn a task with TaskState Ready and update tm.tasks. on every tick, we check whether the currently task with Running state has vruntime + dt > tm.min_vruntime (= min runtime of Ready tasks), if the Running task has updated vruntime > runtime of Ready tasks, then add the Running task back into tm,tasks, have its state = Running, pop the Ready task with min runtime and update tm.vruntime as the next smallest runtime, then I wouldn't have to compute smallest Ready vruntime on every on_tick call, only when the Running task is preempted.

This exploits the nature that the vruntimes of the set of Ready tasks change ONLY if the set is modified. 

How /task/mod.rs works and how context switching works, here we are examining the lifecycle of task_i:

When we spawn a new task_i, we have to obtain the lock of the taskmanager in order to actually be able to write to it w/o race conditions.
so TASK_MANAGER.lock() in spawn()
then find a slot in tasks that is Unused or Exited, and add this new task to that slot

we also need to build the context of this task, which includes ra: addr of which the task's entry or actual executable code is stored

more in depth about ra: task_start as usize, is that ra is actually the addr to the task_start() executable, -> so when this task is running, in asm we have smth like jump(task_start), where in asm this jumps to branch w/ __task_start: ... jump(entry) ... return

task_start works along the lines of:
disable interrupts, no preempting:
lock and get task manager guard, deref to get tm
task_start() is only called when the given task is Running, so it should be accessed via tm.running_id
then we get the fn() entry of that task, 
drop the guard, enable interrupts

actually run entry() until it preempts, or exits -> exit_current()

the idea here is that if we context switch to a different task via yield_now() -> schedule(), the **ra** is set as the addr after the last executed line in the yielded task
you can kind of think about this like recursive stack, eventually when all the prempted / recursed tasks have yieled OR we context switch back to task_i, then we will return back to **ra**, and then continue executing task_i after the last finished line -> until finally exit_current, and task_i.state == TaskState::Exited


sp: top of the kernel_stack that consists of all the task frames: offset to actual task is its index in tasks[] * (KERNEL_STACK_SIZE / MAX_TASKS)

vruntime of this newly spawned task is the min of current Running task's vruntime or min vruntime of Ready tasks: no going below either number

