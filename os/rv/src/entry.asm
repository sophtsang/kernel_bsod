    .section .text.entry
    .globl _start
_start:
    # a0 = hart id, a1 = device tree address (from OpenSBI); call leaves both intact
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
