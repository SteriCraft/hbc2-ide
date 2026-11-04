# ftest — tests of the float library

Runs the routines of `compiler/lib/asm/math/float` on the emulator and checks
them against Rust's `f32` / `f64`. Each case checks the result bit by bit, the
exception flags in `fenv_flags`, and that every register is restored.

```
cargo run --release -- [count] [mode]
```

| mode | what |
|---|---|
| (none) | fadd fsub fdim fmul fdiv fmod |
| `same` | the same, with both operands at one address |
| `misc` `conv` `round` `sqrt` | bits/compare, int conversions, rounding + ldexp/frexp, sqrt |
| `ftoa` `atof` | text conversions |
| `fexp` `flog` `fpow` `fsin` … `fhypot` | one math function. Prints the worst true error in ulp (vs f64) and the stack used. `WORST=1` lists cases above 0.55 ulp, `SHOWU=1` every case that isn't the correctly rounded float |

The driver programs go to `work/`, which git ignores.

Other programs:

- `cargo run --release --bin genk` writes `fmathk.has` (constants and tables of
  the math functions). Don't edit that file by hand.
- `cargo run --release --bin size -- fexp fsin …` prints the bytes a C build
  keeps when it calls only those routines.
