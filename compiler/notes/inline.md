# Inline code (step 6) — how it works

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

All in `gen_shortcut` of backend-hbc2.c (the node is not generated yet: the code picks what to generate first).
- **Operands:** `operand_ok` = constant, T_LREF ([sp + n] ≤ 255), T_NREF/T_LBREF (address), or a CAST of an unsigned char of those (zero-extended: high byte constant 0). ALU instructions use them directly → no push/pop. Commutative ops swap when only the left one is simple.
- **Conditions:** `cc` = which flag state means true (CC_Z, CC_NZ, CC_C, CC_NC; CC_NONE = test the working value). A T_BOOL with CCONLY (if/while/for and && / || parts) calls `cond()`, which leaves flags only. jfalse/jtrue jump on cc (a clear flag = jump over a jmp: no "jump if not" instructions). Parts of && / || (flag CCFIXED) are converted to CC_NZ (`cc_fix`): the jumps of a chain meet at one label. A chain part that is not a T_BOOL (FCC folds `x >= 0u` to a constant) is caught at the top of gen_shortcut. `cc` is reset in gen_node and T_CLEANUP.
- **Comparisons:** always inline (the `__cc*` helpers are now unused by the compiler; the linker will drop them). Ordered: subtract (C = borrow); `a > k` as `a >= k + 1`, otherwise `b - a` with `i` as scratch; signed = flip both sign bits first (`xor 0x80`; a memory byte via `lod j` + `xor j`, which keep C); signed `x < 0` / `x >= 0` = `shl` of the top byte. Equality: xor the bytes, or them together. (int)char vs a constant that fits, or vs a char of the same type: 8-bit compare.
- **Shifts by constants:** 16-bit all counts (k ≥ 8 = byte move; right shift by 1 with asr + the sbb trick; 2..7 = `b >> k | a << (8-k)`); 32-bit only multiples of 8 and `<< 1`.
- **Multiply by a constant** (16-bit): power of 2 = shifts; otherwise shift-and-add when ≤ 12 instructions. **Unsigned / and %** by a power of 2: shifts / and.
- **Pointers:** `*p`, `p->f`: [x:y] loaded straight from the variable (`lod x / lod y`), the constant part of the address in the offset `[x:y + n]`. `*p = v`: no push when the pointer or the value is a variable/constant.
- **++ -- += -= &= |= ^=** on locals/globals/statics: in memory byte by byte through b (unchanged bytes skipped); with a non-constant right value: `add b, [var]` + `str`. Through a pointer: only with a constant (`p->n++`, `(*p)--`); char ±1 in memory = `inc`/`dec` (no `[sp + n]` form). Post-forms keep the old value in the working register, the new one is made in `i`.
- **Bug found by bench.c (from step 3):** a cast that only changes the sign retyped the node below: `s += v % 10` (int s, unsigned v) became a signed %. Now the cast stays when the node below is / % >> or their assignment forms (`sign_matters`).
- **FCC quirk (documented in ABI.md 12):** decimal 32768..65535 is `unsigned int` (C says `long`). gen_ops.py writes such constants with literal().
- Measured with `programs.rs` (prints code bytes = instruction lines × 4, and ticks). Before → after: basics 1564 → 924 bytes, 11816 → 6025 ticks; bench 6200 → 4256 bytes, 1 294 102 → 253 253 ticks.
- Mutation check done (7 mutations of the new code, each fails ≥ 1 program).
- Left for step 7 (copt): `jmc X / jmp L / X: jmp M` (FCC's for-loop layout), `jmp L` + `L:` (every function's return), jump to a jump, `str` then `lod` of the same place, the same constant loaded twice (`mov a, 0 / mov b, 0`). All done in step 7 except the last (`mov b, a` costs the same).

