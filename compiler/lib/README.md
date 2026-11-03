# HBC-2 C library

What C programs can `#include`, and the code behind it. The compiler puts
`include/` on the include path; every program is assembled with `lib.has`
(and the compiled `src/*.c` files, when there are some); only what the program uses is kept (see
"Size"). `lib.has` and the generated `library.has` start with `.library`:
a program may define a library name (`puts`, `putchar`…) and its own
definition is used everywhere, the library's left out.

| Header | Content | Code |
|---|---|---|
| `hbc2.h` | device ports, `in`/`out`, interrupts, monitor, terminal, RTC, keyboard, EEPROM (`eeprom_read`, `eeprom_write`, `eeprom_load`, `eeprom_save`), random-number extras (`srand_rtc`, `rand_range`) | `asm/hbc2.has` (wrappers; the EEPROM functions are written there) |
| `stdio.h` | `putchar`, `puts`, `printf`, `vprintf`, `perror` (output on the terminal), `sprintf`, `vsprintf` (into a buffer), `getchar`, `gets`, `scanf` (lines typed), `sscanf` (from a string) | `asm/stdio.has`, `asm/printf.has`, `asm/scanf.has`, `asm/errno.has` (`perror`) |
| `stdarg.h` | `va_list`, `va_start`, `va_arg`, `va_end` | |
| `string.h` | `strlen strcpy strncpy strcat strncat strcmp strncmp strchr strrchr strstr strspn strcspn strpbrk strtok memcpy memmove memset memcmp memchr strerror` | `asm/string.has`, `asm/errno.has` (`strerror`) |
| `errno.h` | `errno`, `EDOM`, `ERANGE` (set by `strtol`, `strtoul`) | `asm/errno.has` |
| `time.h` | `time_t` (seconds since 1970, unsigned: to 2106), `struct tm`, `time`, `clock` (always -1), `mktime`, `localtime`, `gmtime` (the same: no time zone), `asctime`, `ctime`, `strftime` (C89 conversions and `%e`); no `difftime` (a `double`) | `asm/time.has` |
| `limits.h` | `CHAR_BIT`, `MB_LEN_MAX`, the `_MIN`/`_MAX` of `char`, `short`, `int`, `long` | |
| `ctype.h` | `isalnum isalpha islower isupper isdigit isxdigit isspace isblank ispunct iscntrl isprint isgraph tolower toupper` | `asm/ctype.has` |
| `stddef.h` | `NULL`, `size_t`, `ptrdiff_t`, `offsetof` | |
| `stdlib.h` | `exit`, `abs`, `labs`, `rand`, `srand`, `RAND_MAX`, `atoi`, `atol`, `strtol`, `strtoul`, `qsort`, `bsearch`, `malloc`, `free`, `calloc`, `realloc`; common extras: `itoa utoa ltoa ultoa` (radix 2, 8, 10, 16) | `asm/stdlib.has`, `asm/sort.has`, `asm/malloc.has`, `asm/math/misc` |
| `math.h` | integer only (not standard C): `isqrt lsqrt ipow lpow gcd lgcd popcount clz ctz lpopcount lclz lctz min max umin umax` | `asm/math.has` (wrappers), `asm/math` |

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
* `asm/math/` — **copy** of the math library,
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
* `asm/stdio.has` — `putchar`, `puts`, `getchar`, `gets` (in C until
  2026-11-03). The input keeps one character looked at but not used
  (`lib_stdin_back`): `scanf` leaves there the character after its last
  field, and `getchar` gives it first.
* `asm/scanf.has` — `scanf`, `sscanf`: one engine (about 1.7 KB), its
  state in 26 bytes of locals. Where the characters come from is
  `lib_scan_src` (0: the keyboard, else the string). The keyboard is
  reached through `lib_scan_key`, set by `scanf`, so a program with only
  `sscanf` doesn't carry the terminal and screen drivers (2 KB instead
  of 3.7 KB). Numbers: 32 bits, one `lib_tm_muladd` (`v * base + digit`,
  `asm/time.has`) a digit. `scanf` reads the keyboard through
  `lib_stdin_peek`, not `getchar`: a program's own `getchar` is not used.
* `asm/printf.has` — `printf`, `vprintf`, `sprintf`, `vsprintf`: one
  engine (in C until 2026-11-03: 2.5 KB, now about 1.1 KB). Its state is in
  30 bytes of locals on the stack. Where the characters go is
  `lib_printf_out` (2 bytes of `.bss`): 0 for `_putchar` (so a program's own
  `putchar` is used), else the next byte of the buffer. The engine keeps
  the caller's value and puts it back, so a `sprintf` inside a `printf`
  (a program's `putchar` calling it) works.
* `asm/ctype.has` — `ctype.h`, written by the project's author: range
  compares on the low byte, false (or unchanged) when the high byte is
  not 0 (`EOF`). The `is…` functions share their exits (`ctype_true`,
  `ctype_false`); `isgraph` jumps to `isprint` (a tail call). 668 bytes
  in all, 9 to 24 instructions a call.
* `asm/time.has` — `time.h`. The core: a date to days since 1970 and
  back (loops over the years and months; 2100 is not a leap year), and
  32-bit multiply-add and divide by a byte (`v * 24 + hour`…, `v / 60`…).
  `mktime` puts the fields in range by carrying (61 s = 1 min 1 s, day 0 =
  the last day of the month before) before it converts. `time` + `localtime`
  ≈ 1 KB; with `mktime`, `strftime`, `asctime` ≈ 2.6 KB and 0.2 KB of
  names and formats. `strftime` keeps its state in 6 bytes of `.bss`.
* `asm/sort.has` — `qsort` (Shell sort, Knuth's gaps 1, 4, 13, 40…: no
  recursion, no memory taken; 410 bytes) and `bsearch` (183 bytes); they
  share a 16-bit multiplication (40 bytes). The comparison function is
  called through `cal [x:y]`.
* `asm/malloc.has` — `malloc`, `free`, `calloc`, `realloc` (about 700
  bytes for the four; 2 bytes of `.bss`, `lib_heap_top`). The heap starts
  at `__bss_end` and grows toward the stack, keeping 256 bytes free below
  SP (`LIB_HEAP_MARGIN`): a chain of blocks with a 2-byte header (the
  size, header included, rounded up to even; bit 0 = used). `free` only
  clears bit 0; `malloc` takes the first free block big enough, joining
  the free blocks that follow each other as it goes, and cuts the rest
  into a new free block; free blocks at the end bring the top down.
  `realloc` grows in place into the free blocks after it, or at the top,
  else moves. The search walks every block: slower as the count grows.
* `asm/string.has` also has `strspn`, `strcspn`, `strpbrk` (one scan,
  `lib_str_span`: 152 bytes together) and `strtok` (118 bytes; where it
  goes on is `lib_strtok_next`, 2 bytes of `.bss`).
* `asm/errno.has` — `errno` (a `.common`, so a program's `int errno;`
  is the same variable), `strerror` (the texts one after the other: it
  skips as many as the number; a new error goes before "Unknown error"),
  `perror` (through `_putchar`, like `printf`).
* `asm/stdlib.has` — `exit`, `abs`, `labs`, the random-number wrappers,
  `itoa`, `utoa`, `ltoa`, `ultoa` (radix 10 from the math library's
  routines; 2, 8 and 16 from the top bits, shifted out to the left) and
  `strtol`, `strtoul`, `atoi`, `atol` (bases 0 and 2 to 36; one
  shift-and-add multiplication a digit, which reports the overflow for
  `ERANGE`; no division anywhere). In C until 2026-11-03 (`src/stdlib.c`):
  the conversion test went from 12000 to 9036 bytes.
* `src/*.c` — library functions written in C, compiled by the build. None
  since 2026-11-03 (`strstr` was the last): the folder is gone, and the
  build still takes C files if it comes back
  (`hbc2_cc::build_library`). Unit numbers from 100 (`__L100_…`), so their
  labels and `static` names don't clash with the program's files (1, 2, …).
  A file's number is its place in name order: when a file is added or
  removed, the build compiles them all again.

Device detection (`monitor_port`, …) is in the startup code:
`../runtime/devices.has`, included by `../runtime/crt0.has` (RAM) and
`../runtime/boot.has` (EEPROM; see `../ABI.md`, section 8).

`printf` has the minimal set: `%d %u %x %X %c %s %%`, `l` for `long`
(`%ld %lu %lx %lX`), a width, `0` for zero padding (`%04x`), `-` to align left
(`%-6s`), widths up to 255. No floating point. It gets its digits from
the math library's `utoa16`, `itoa16`, `htoa16` and their 32-bit forms
(`asm/math/misc/text.has`; no division).

## Size

In all, about 6.3 KB of code and data: the drivers and wrappers ≈ 6 KB
(terminal 1.6 KB, screen 1.3 KB, `string.h` 1.1 KB, `printf` 1 KB,
wrappers 0.7 KB). The
runtime helpers add up to ≈ 6.5 KB more (multiplication 1.4 KB,
division 2.2 KB, shifts 0.8 KB: only those a program uses). `string.h` was in C until 2026-10-31
(2.7 KB, 2-4 times slower), `printf`, `strstr` and the `stdlib.h`
conversions until 2026-11-03.

The assembler leaves out what a program can't reach (assembly manual,
"Leaving out unused code"), so a program carries only what it uses: an
empty `main` is 269 bytes (startup, device detection, `exit`), hello world
with `puts` 1.1 KB (the terminal and screen drivers), with `printf`
2.7 KB (3.8 KB with the C `printf`). `hbc2 cc … --map` shows what was kept.

## Tests

`compiler/tests/hardware.c`, `eeprom.c`, `stdio.c`, `strings.c`, `printf.c`,
`random.c`, `convert.c`, `math.c`, `ctype.c`, `malloc.c`, `scanf.c` (some
with `.screen` / `.keys` files) check the library on the emulator: see
`../tests/README.md`.
