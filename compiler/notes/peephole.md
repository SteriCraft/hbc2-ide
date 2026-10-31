# Peephole rules (step 7) — how it works

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- `compiler/fcc/rules.hbc2`, run by `copt` (bin/copt.exe, Fraser's, from Z88DK) after cc2, in both drivers (`hbc2_cc::compile`, hbc2cc.py). The file's header explains the format and what the rules rely on.
- copt matches backwards from each line, in rule order; `%n` = text up to the next pattern char; `%eval R = rpn` / `%check lo <= %n <= hi` are conditions (put them first: checked after the lines match); `%eval(rpn)` in a replacement; `%activate` makes rules at run time (used for jump threading: `L: jmp M` → every `jm? L` becomes `jm? M`; `L: ret` → `jmp L` becomes `ret`). Only `%activate` starts another pass (max 16); a rule fires at most 65535 times, which also ends jump cycles (`a: goto b; b: goto a;` checked: fast, code right).
- Rules: for-loop `jm? __X / jmp B / __X: jmp N` → `jm? N / jmp B`; jump to the next line; threading; tail call `cal f / L_r: / ret` → `jmp f`; dead code after jmp; store then load of the same place (removed in step 12: reloads.rs does it and knows volatiles); `psh b, a / 2 loads / pop d, c` → `mov c, a / mov d, b / loads` (stack offsets −2); redundant movs.
- Coverage: each rule was removed in turn and the 52 outputs compared; every rule changes at least one (except the `mov r, r` guard). Mutation check: 6 broken rules, each fails ≥ 1 program (peephole.c's `longs` was extended for this).
- programs.rs now counts `psh`/`pop` lists as one instruction per register (the numbers above use this count).
- Tried and left out (never fired in 52 programs): jump to a label after other labels, `jm? L / jmp L`, dead code after `ret`, `cal f / ret` without a label.

