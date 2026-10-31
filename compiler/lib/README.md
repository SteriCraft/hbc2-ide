# HBC-2 C library

What C programs can `#include`, and the code behind it. The compiler puts
`include/` on the include path; every program is assembled with `lib.has`
and the compiled `src/*.c` files; only what the program uses is kept (see
"Size"). `lib.has` and the generated `library.has` start with `.library`:
a program may define a library name (`puts`, `putchar`…) and its own
definition is used everywhere, the library's left out.

| Header | Content | Code |
|---|---|---|
| `hbc2.h` | device ports, `in`/`out`, interrupts, monitor, terminal, RTC, keyboard | `asm/hbc2.has` (wrappers) |
| `stdio.h` | `putchar`, `puts`, `printf` (output on the terminal), `getchar`, `gets` (lines typed) | `asm/stdio.has`, `src/stdio.c`, `src/printf.c` |
| `stdarg.h` | `va_list`, `va_start`, `va_arg`, `va_end` | |
| `string.h` | `strlen strcpy strncpy strcat strncat strcmp strncmp strchr strrchr memcpy memmove memset memcmp memchr` | `asm/string.has` |
| `stddef.h` | `NULL`, `size_t`, `ptrdiff_t`, `offsetof` | |
| `stdlib.h` | `exit` | `asm/stdlib.has` |

## Files

* `include/` — the headers.
* `lib.has` — includes the assembly files below.
* `asm/screen.has`, `asm/terminal.has` — **copies** of the monitor driver
  and the terminal of `design_projects/ASM_p/lib/monitor`. The
  changes are listed at the top of each file (the port variables have their
  C names). When the originals change, copy them again and redo the
  changes.
* `asm/hbc2.has` — the C wrappers: they read the C arguments from the stack
  (`../ABI.md`, section 9) and call the driver routines.
* `asm/stdio.has` — `putchar`, `puts`.
* `asm/stdlib.has` — `exit`.
* `src/*.c` — library functions written in C, compiled by the build
  (`hbc2_cc::build_library`). Unit numbers from 100 (`__L100_…`), so their
  labels and `static` names don't clash with the program's files (1, 2, …).

Device detection (`monitor_port`, …) is in the startup code:
`../runtime/devices.has`, included by `../runtime/crt0.has`.

`printf` has the minimal set: `%d %u %x %c %s %%`, `l` for `long`
(`%ld %lu %lx`), a width, `0` for zero padding (`%04x`, `%-5d` is not
there). No floating point.

## Size

In all, about 7.7 KB of code and data: the drivers and wrappers ≈ 5 KB
(terminal 1.6 KB, screen 1.3 KB, `string.h` 1.1 KB, wrappers 0.7 KB), the
compiled C files ≈ 2.8 KB (`printf.c` 2.5 KB, `stdio.c` 0.3 KB). The
runtime helpers add ≈ 3.9 KB more. `string.h` was in C until 2026-10-31
(2.7 KB, 2-4 times slower).

The assembler leaves out what a program can't reach (assembly manual,
"Leaving out unused code"), so a program carries only what it uses: an
empty `main` is 340 bytes (startup, device detection, `exit`), hello world
with `puts` 1.7 KB (the terminal and screen drivers), with `printf`
4.8 KB. `hbc2 cc … --map` shows what was kept.

## Tests

`compiler/tests/hardware.c`, `stdio.c`, `strings.c`, `printf.c` (some
with `.screen` / `.keys` files) check the library on the emulator: see
`../tests/README.md`.
