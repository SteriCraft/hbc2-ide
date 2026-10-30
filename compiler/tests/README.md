# C compiler tests

Each file says what `main` returns. Until the test harness exists (roadmap
step 5), run one by hand:

```
python ../tools/hbc2cc.py basics.c -o basics.has
```

then assemble and run a `.has` file that includes, in this order,
`../runtime/crt0.has`, an interrupt handler file (for example the test
project's `interruptHandler.has`) and `basics.has`:

```
cargo run -p hbc2-cli -- run run.has --ticks 2000000
```

`main`'s result is in `[a:b]` when the CPU waits at `__exit`.
