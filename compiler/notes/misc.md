# Notes (CPU facts, FCC quirks, misc)

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- FCC file name bug (fixed, in UPSTREAM.md): cc0 cut cpp file names at 32 chars keeping what follows the last '/', cc1 lost sync on a 32-char name → "useless declaration". Now '\' counts too, and cc1 reads the end marker.
- hbc2-asm `.include` paths: use '/' (a '\' starts an escape) — the tests convert paths.

- `backend-default.c` assumes every push is a word (`get_stack_size` 1 → 2): with real 1-byte char arguments (target_argsize = size) its sp check fails ("error: sp"). Our backend must count real sizes. FCC pushes a `char` argument as 1 byte with a `char` prototype, as an int in variadic calls (checked on 6809).
- ABI test program (add3 / screen_putc wrapper / interrupt calling C): scratchpad `abi/abi.has`, run with `cargo run -p hbc2-cli -- run abi.has --ticks 200000` → a=00 b=01 c=00 d=5a i=27 j=34 x=11 y=22, sp=fe00. Rewrite it in the step 5 harness.
- CPU facts used by the ABI: reset enables interrupts; interrupt entry pushes i then PC (no flags), i = data, I cleared; irt pops PC, i, sets I; hlt sets Halt AND enables interrupts (can't stop for good → `__exit` loops); mov/lod/str/psh/pop/cal/ret keep flags; cmp sets S/E/F only.
- Fuzix names were cut at 15 chars: raised to 31 (NAMELEN 32) — the one upstream change, listed in fcc/UPSTREAM.md.

- `[sp + n]`: n = 0..255; `[sp]` = last byte pushed. Track SP changes from `psh` in the backend (Fuzix backends already track an sp offset).
- Address of local at offset n: `rsp [x:y]`, `add y, n`, `adc x, 0` (16-bit add).
  For n > 255 only to access: `rsp [x:y]`, `add x, n >> 8`, then `[x:y + (n & 255)]` (the CPU adds the offset on 16 bits, so no carry handling).
- No 16-bit ALU ops: 16-bit add = `add` low + `adc` high.
- Function pointers: `cal [r:r]`.
- Math library: `design_projects/test_project/lib/math` (user's files; 16/ in registers, 32/ through pointers). Don't edit without saying so in chat.

