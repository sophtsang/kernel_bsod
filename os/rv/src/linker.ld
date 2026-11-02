OUTPUT_ARCH(riscv)
ENTRY(_start)

/* OpenSBI jumps here in S-mode */
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .bss : ALIGN(4K) {
        /* stack sits before sbss so clear_bss doesn't zero the stack it's running on */
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
    }

    /DISCARD/ : {
        *(.eh_frame)
    }
}
