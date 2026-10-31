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
* `random.c`, `convert.c`: `stdlib.h` random numbers and number <-> text.
* `math.c`: `math.h`, `abs`, `labs`, each against plain C on edge and
  pseudo-random values (also `sizeof a / sizeof a[0]`).
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
