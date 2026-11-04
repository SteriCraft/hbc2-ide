# C compiler tests

Each file says what `main` returns on its first line; "built for the
EEPROM" on that line builds it for the EEPROM and starts it from there
(no RAM image). A test that uses
the monitor or the keyboard can have, next to `name.c`, a `name.screen`
file (the text the monitor shows at the end; rows without trailing spaces,
empty rows at the end left out) and a `name.keys` file (text typed once
the program reads keys; a new line is Enter). Programs run with a monitor,
an RTC (at 2000-01-01 00:00:00, not moving) and a keyboard. The Rust test
`hbc2/crates/hbc2-cc/tests/programs.rs` compiles and runs them all
(`cargo test -p hbc2-cc`, after `compiler/build.cmd`), and prints the size
of each program's code and the number of instructions it ran.

* `basics.c`, `frames.c`: calls, structs, pointers, big stack frames.
* `bench.c`: small real programs (sieve, sort, strings, CRC, `long`
  arithmetic); also the benchmark for the code the compiler makes.
* `hardware.c`: `hbc2.h` (device ports, `in`/`out`, monitor text and
  pixels, RTC, keyboard, EEPROM functions refused without an EEPROM) and
  `exit`.
* `eeprom.c`: built for the EEPROM (its first line says so) and started
  from it: the boot loader, `eeprom_read`/`write`/`load`/`save` (64 KiB and
  256-byte boundaries, the last byte, past the end).
* `stdio.c`: `putchar`, `puts`, `getchar`, `gets`, scrolling.
* `strings.c`: `string.h`, with lengths over 255 and overlapping moves.
* `ctype.c`: `ctype.h`, every function on every value from `EOF` to 255
  and a few out of range, against plain C (the `is…` must give exactly
  0 or 1).
* `printf.c`: every `printf` format, widths and padding, the counts.
* `sprintf.c`: `sprintf`, `vsprintf`, `vprintf` (the formats of
  `printf.c`, the final 0, a `sprintf` inside a `printf`, through the
  program's own `putchar`).
* `printf_float.c`: `%f %e %g` (and capitals) against the host's printf on
  29 values x 25 formats (0, -0, subnormals, the largest float, inf, nan,
  halfway cases), the integer precision and flags, `*`, more than 192
  digits; `sscanf` `%f %e %g %E`, `l`, `L`, widths, `*`, failures.
* `sort.c`: `qsort` (0 to 300 elements: permutations, sorted, reversed,
  equal values; 5-byte records, strings; the comparison function checks
  every pointer it gets), `bsearch` (every element found, the others
  not), `strspn`, `strcspn`, `strpbrk`, `strtok`.
* `time.c`: `time.h`: well-known dates (2038, 2100 not a leap year, the
  last second of 2106), 300 round trips against plain C, `mktime` putting
  fields in range and its limits, `asctime`, `ctime`, every `strftime`
  conversion, `max`.
* `scanf.c` (with `scanf.keys`): `sscanf`: every conversion, signs,
  bases and `%i` prefixes, 32-bit values, widths, sets (ranges, `^`, `]`
  first), literals, `%%`, `*`, `%n`, the results on failure (0, EOF,
  the count so far); `scanf` from typed lines, the character it leaves
  for `getchar`, reading on into the next line, then `gets`.
* `malloc.c`: `malloc`, `free`, `calloc`, `realloc`: blocks apart, a
  freed block used again, first fit and the rest cut off, free blocks
  joined, `calloc` set to 0 and its overflow, too big, `realloc` in place
  (shrinking, the free blocks after it, at the top) or moved (the old
  block free again; failure keeps it), the whole heap filled (256 bytes
  kept below the stack) and given back, 600 random steps checking every
  block's bytes.
* `structcopy.c`: struct and union assignment: sizes 1 to 300 bytes
  (inline and `__structcpy`), globals, locals, through pointers, arrays of
  structs (the address computed once), `a = b = c`, `struct s v = w;`,
  nested structs.
* `localinit.c`: initial values of local arrays and structs (`{ }`,
  strings, sizes from the values, the rest at 0, nested, 2D), copied
  again at each pass and in recursion; `(void)` casts.
* `random.c`, `convert.c`: `stdlib.h` random numbers and number <-> text.
* `errno.c`: `limits.h` (values and types), `errno` set by `strtol` and
  `strtoul`, `strerror`, `perror` (`errno.screen`).
* `math.c`: `math.h`, `abs`, `labs`, each against plain C on edge and
  pseudo-random values (also `sizeof a / sizeof a[0]`).
* `float.c`: `float` and `double`: constants (cc0, every form: `.5`, `5.`,
  `0x1.8p1`, subnormals), folding, the operators, -0.0 and NaN in
  conditions, `++`/`--`, every conversion to and from the integer types,
  structs, arrays, pointers, arguments and results (checked bit by bit).
* `fmath.c`: the float `math.h` functions, `float.h`, `fenv.h` (the flags
  of each operation, `feholdexcept`...), `strtod`, `strtof`, `atof`.
* `peephole.c`: code shapes for the peephole rules (`../fcc/rules.hbc2`)
  that the other programs reach rarely.
* `multi/<name>/`: a program of several files: its `.c` and `.has` files,
  built together in name order (`main.c` says what `main` returns, and
  `main.screen` / `main.keys` go next to it). `multi/vector`: `set_vector`
  with a key handler in assembly that calls C. `multi/files`: names shared
  through a header, the same `static` names in two files, a static
  function called through a pointer from the other file, a function
  written in assembly. Builds that must fail (a name defined twice, a name
  defined nowhere) are in `hbc2/crates/hbc2-cc/tests/build.rs`.
* `ops/*.c` come from `gen_ops.py`: every operator on one type, with
  variables and constants, as values and as conditions, on globals (`_g`)
  and locals (`_l`). They return the number of the first failed check, 0
  when all pass. Run `python gen_ops.py` again after changing it.

To run one by hand:

```
python ../tools/hbc2cc.py basics.c -o basics.has
```

(`--no-copt` gives the code without the peephole rules, to compare.)

then assemble and run a `.has` file that includes, in this order,
`../runtime/crt0.has` (startup and device detection),
`../runtime/runtime.has`, `../lib/lib.has`, the library's C files compiled
with `--unit 100`, `--unit 101`, … (`../lib/src/*.c`) and `basics.has`
(the files of a `multi` program: with `--unit 1`, `--unit 2`, …):

```
cargo run -p hbc2-cli -- run run.has --ticks 2000000
```

`main`'s result is in `[a:b]` when the CPU waits at `__exit`.
