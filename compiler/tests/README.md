# C compiler tests

Each file says what `main` returns on its first line. The Rust test
`hbc2/crates/hbc2-cc/tests/programs.rs` compiles and runs them all
(`cargo test -p hbc2-cc`, after `compiler/build.cmd`), and prints the size
of each program's code and the number of instructions it ran.

* `basics.c`, `frames.c`: calls, structs, pointers, big stack frames.
* `bench.c`: small real programs (sieve, sort, strings, CRC, `long`
  arithmetic); also the benchmark for the code the compiler makes.
* `peephole.c`: code shapes for the peephole rules (`../fcc/rules.hbc2`)
  that the other programs reach rarely.
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
`../runtime/crt0.has`, an interrupt handler file (for example the test
project's `interruptHandler.has`) and `basics.has`:

```
cargo run -p hbc2-cli -- run run.has --ticks 2000000
```

`main`'s result is in `[a:b]` when the CPU waits at `__exit`.
