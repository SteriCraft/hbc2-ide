# HBC-2 C library

What C programs can `#include`, and the code behind it. The compiler puts
`include/` on the include path; every program is assembled with `lib.has`
and the compiled `src/*.c` files; only what the program uses is kept (see
"Size"). `lib.has` and the generated `library.has` start with `.library`:
a program may define a library name (`puts`, `putchar`…) and its own
definition is used everywhere, the library's left out.

| Header | Content | Code |
|---|---|---|
| `hbc2.h` | device ports, `in`/`out`, interrupts, monitor, terminal, RTC, keyboard, EEPROM (`eeprom_read`, `eeprom_write`, `eeprom_load`, `eeprom_save`) | `asm/hbc2.has` (wrappers; the EEPROM functions are written there) |
| `stdio.h` | `putchar`, `puts`, `printf` (output on the terminal), `getchar`, `gets` (lines typed) | `asm/stdio.has`, `src/stdio.c`, `src/printf.c` |
| `stdarg.h` | `va_list`, `va_start`, `va_arg`, `va_end` | |
| `string.h` | `strlen strcpy strncpy strcat strncat strcmp strncmp strchr strrchr memcpy memmove memset memcmp memchr` | `asm/string.has` |
| `ctype.h` | `isalnum isalpha islower isupper isdigit isxdigit isspace isblank ispunct iscntrl isprint isgraph tolower toupper` | `asm/ctype.has` |
| `stddef.h` | `NULL`, `size_t`, `ptrdiff_t`, `offsetof` | |
| `stdlib.h` | `exit`, `abs`, `labs`, `rand`, `srand`, `RAND_MAX`, `atoi`, `atol`; extras: `srand_rtc`, `rand_range`, `itoa utoa ltoa ultoa htoa lhtoa` | `asm/stdlib.has`, `asm/mathlib/misc` |
| `math.h` | integer only (not standard C): `isqrt lsqrt ipow lpow gcd lgcd popcount clz ctz lpopcount lclz lctz min max umin umax` | `asm/math.has` (wrappers), `asm/mathlib` |

`div`, `ldiv` (stdlib.h) are missing: they return a struct, and FCC has no
struct return. `math.h` keeps the standard names (`sqrt`, `pow`…) free
for floating point one day.

## Files

* `include/` — the headers.
* `lib.has` — includes the assembly files below.
* `asm/screen.has`, `asm/terminal.has` — **copies** of the monitor driver
  and the terminal of `design_projects/ASM_p/lib/monitor`. The
  changes are listed at the top of each file (the port variables have their
  C names). When the originals change, copy them again and redo the
  changes.
* `asm/mathlib/` — **copy** of the math library,
  `design_projects/ASM_p/lib/math`, without the 6 files C never reaches
  (signed arithmetic, logic and shifts: listed at the top of its
  `math.has`, which no longer includes them); `lib.has` includes its
  `math.has`. When the originals change, copy the folder again and redo
  that. Its
  routines save the registers they don't return, so the wrappers just
  load the arguments and call them.
* `asm/math.has` — the `math.h` wrappers. The 32-bit routines work on the
  `long` argument itself, on the stack (`[x:y]` points to it).
* `asm/hbc2.has` — the C wrappers: they read the C arguments from the stack
  (`../ABI.md`, section 9) and call the driver routines.
* `asm/stdio.has` — `putchar`, `puts`.
* `asm/ctype.has` — `ctype.h`, written by the project's author: range
  compares on the low byte, false (or unchanged) when the high byte is
  not 0 (`EOF`). The `is…` functions share their exits (`ctype_true`,
  `ctype_false`); `isgraph` jumps to `isprint` (a tail call). 668 bytes
  in all, 9 to 24 instructions a call.
* `asm/stdlib.has` — `exit`, `abs`, `labs`, the random-number and the
  number <-> text wrappers.
* `src/*.c` — library functions written in C, compiled by the build
  (`hbc2_cc::build_library`). Unit numbers from 100 (`__L100_…`), so their
  labels and `static` names don't clash with the program's files (1, 2, …).

Device detection (`monitor_port`, …) is in the startup code:
`../runtime/devices.has`, included by `../runtime/crt0.has` (RAM) and
`../runtime/boot.has` (EEPROM; see `../ABI.md`, section 8).

`printf` has the minimal set: `%d %u %x %X %c %s %%`, `l` for `long`
(`%ld %lu %lx %lX`), a width, `0` for zero padding (`%04x`, `%-5d` is not
there). No floating point. It gets its digits from `utoa`, `ultoa`, `htoa` and
`lhtoa` (no division: 3.5 times faster than with `/` and `%`, for 0.3 KB
more).

## Size

In all, about 7.7 KB of code and data: the drivers and wrappers ≈ 5 KB
(terminal 1.6 KB, screen 1.3 KB, `string.h` 1.1 KB, wrappers 0.7 KB), the
compiled C files ≈ 2.8 KB (`printf.c` 2.5 KB, `stdio.c` 0.3 KB). The
runtime helpers add up to ≈ 6.5 KB more (multiplication 1.4 KB,
division 2.2 KB, shifts 0.8 KB: only those a program uses). `string.h` was in C until 2026-10-31
(2.7 KB, 2-4 times slower).

The assembler leaves out what a program can't reach (assembly manual,
"Leaving out unused code"), so a program carries only what it uses: an
empty `main` is 340 bytes (startup, device detection, `exit`), hello world
with `puts` 1.7 KB (the terminal and screen drivers), with `printf`
5.1 KB. `hbc2 cc … --map` shows what was kept.

## Tests

`compiler/tests/hardware.c`, `eeprom.c`, `stdio.c`, `strings.c`, `printf.c`,
`random.c`, `convert.c`, `math.c`, `ctype.c` (some
with `.screen` / `.keys` files) check the library on the emulator: see
`../tests/README.md`.
