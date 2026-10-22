MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 192K
}

__stack_top = ORIGIN(RAM) + LENGTH(RAM);
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
_stack_end = ORIGIN(RAM);

SECTIONS
{
  .isr_vector :
  {
    . = ALIGN(4);
    __isr_vector = .;
    LONG(ORIGIN(RAM) + LENGTH(RAM));          // Initial SP
    LONG(Reset);                               // Reset handler
    . = ALIGN(4);
  } >FLASH

  .text :
  {
    . = ALIGN(4);
    *(.text*)
    *(.rodata*)
    . = ALIGN(4);
  } >FLASH

  .data :
  {
    . = ALIGN(4);
    __sdata = .;
    *(.data*)
    *(.data.*)
    . = ALIGN(4);
    __edata = .;
  } >RAM AT >FLASH
  __sidata = LOADADDR(.data);

  .bss :
  {
    . = ALIGN(4);
    __sbss = .;
    *(.bss*)
    *(.bss.*)
    *(COMMON)
    . = ALIGN(4);
    __ebss = .;
  } >RAM

  /DISCARD/ :
  {
    *(.ARM.exidx*)
  }
}

ENTRY(Reset);
