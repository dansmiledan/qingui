MEMORY
{
  FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 16M
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 4M
}

_stack_size = 64K;
