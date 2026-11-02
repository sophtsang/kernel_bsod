# kernel trap entry/exit. stvec points at __trap_entry, so every trap and interrupt lands here.
#
# frame layout = TrapContext in context.rs (34 * 8 = 272 bytes, a multiple of 16):
#   x[i]    at i*8       i = 0..31  (x0 slot unused; x2 holds sp from *before* the trap)
#   sstatus at 32*8 = 256
#   sepc    at 33*8 = 264
#
# kernel-only for now: the frame goes on whatever stack was running. phase 3a adds the
# sscratch swap so traps from user mode land on the kernel stack instead.

    .section .text
    .globl __trap_entry
    .align 2                    # stvec direct mode needs a 4-byte aligned address
__trap_entry:
    addi sp, sp, -34*8

    sd x1,   1*8(sp)
    # x2 (sp) saved below, once a temp register is free to compute its old value
    sd x3,   3*8(sp)
    sd x4,   4*8(sp)
    sd x5,   5*8(sp)
    sd x6,   6*8(sp)
    sd x7,   7*8(sp)
    sd x8,   8*8(sp)
    sd x9,   9*8(sp)
    sd x10, 10*8(sp)
    sd x11, 11*8(sp)
    sd x12, 12*8(sp)
    sd x13, 13*8(sp)
    sd x14, 14*8(sp)
    sd x15, 15*8(sp)
    sd x16, 16*8(sp)
    sd x17, 17*8(sp)
    sd x18, 18*8(sp)
    sd x19, 19*8(sp)
    sd x20, 20*8(sp)
    sd x21, 21*8(sp)
    sd x22, 22*8(sp)
    sd x23, 23*8(sp)
    sd x24, 24*8(sp)
    sd x25, 25*8(sp)
    sd x26, 26*8(sp)
    sd x27, 27*8(sp)
    sd x28, 28*8(sp)
    sd x29, 29*8(sp)
    sd x30, 30*8(sp)
    sd x31, 31*8(sp)

    # t0-t2 are saved, so they're free to use now
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    addi t0, sp, 34*8           # sp as it was when the trap hit
    sd t0, 2*8(sp)

    mv a0, sp                   # trap_handler(ctx: &mut TrapContext)
    call trap_handler

    # the handler may have changed ctx.sepc (e.g. to skip an ebreak), so reload both
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1

    ld x1,   1*8(sp)
    # x2 (sp) restored last, by popping the frame
    ld x3,   3*8(sp)
    ld x4,   4*8(sp)
    ld x5,   5*8(sp)
    ld x6,   6*8(sp)
    ld x7,   7*8(sp)
    ld x8,   8*8(sp)
    ld x9,   9*8(sp)
    ld x10, 10*8(sp)
    ld x11, 11*8(sp)
    ld x12, 12*8(sp)
    ld x13, 13*8(sp)
    ld x14, 14*8(sp)
    ld x15, 15*8(sp)
    ld x16, 16*8(sp)
    ld x17, 17*8(sp)
    ld x18, 18*8(sp)
    ld x19, 19*8(sp)
    ld x20, 20*8(sp)
    ld x21, 21*8(sp)
    ld x22, 22*8(sp)
    ld x23, 23*8(sp)
    ld x24, 24*8(sp)
    ld x25, 25*8(sp)
    ld x26, 26*8(sp)
    ld x27, 27*8(sp)
    ld x28, 28*8(sp)
    ld x29, 29*8(sp)
    ld x30, 30*8(sp)
    ld x31, 31*8(sp)

    addi sp, sp, 34*8
    sret
