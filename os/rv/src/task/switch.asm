# __switch(current: *mut TaskContext, next: *const TaskContext)
#
# saves the running task's callee-saved registers into *current, loads next's, and
# returns into next. it's an ordinary function call, so the compiler has already
# saved every caller-saved register around it; only ra, sp, s0-s11 need saving.
#
# layout = TaskContext in task/context.rs (14 * 8 = 112 bytes):
#   ra at 0, sp at 8, s0..s11 at 16 + i*8

    .section .text
    .globl __switch
    .align 2
__switch:
    sd ra,   0*8(a0)
    sd sp,   1*8(a0)
    sd s0,   2*8(a0)
    sd s1,   3*8(a0)
    sd s2,   4*8(a0)
    sd s3,   5*8(a0)
    sd s4,   6*8(a0)
    sd s5,   7*8(a0)
    sd s6,   8*8(a0)
    sd s7,   9*8(a0)
    sd s8,  10*8(a0)
    sd s9,  11*8(a0)
    sd s10, 12*8(a0)
    sd s11, 13*8(a0)

    ld ra,   0*8(a1)
    ld sp,   1*8(a1)
    ld s0,   2*8(a1)
    ld s1,   3*8(a1)
    ld s2,   4*8(a1)
    ld s3,   5*8(a1)
    ld s4,   6*8(a1)
    ld s5,   7*8(a1)
    ld s6,   8*8(a1)
    ld s7,   9*8(a1)
    ld s8,  10*8(a1)
    ld s9,  11*8(a1)
    ld s10, 12*8(a1)
    ld s11, 13*8(a1)

    # jumps to next's saved ra: where it last called __switch, or, for a task that
    # has never run, the entry point new_task() stored there
    ret
