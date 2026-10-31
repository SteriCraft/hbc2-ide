# Step 11: linker (Claude's tracking file)

Goal: a C program carries only the code and data it can reach (library, runtime helpers, its own files). Today every program carries ≈ 13.2 KB (library 9.3 KB + runtime 3.9 KB).
Decision 15 in `ROADMAP.md`: no object files. hbc2-asm removes what can't be reached. Agreed with the user on 2026-10-31.
Continue at **Next**, tick the boxes, and add notes under "How it works" as the work goes (keep them short).

## Status

- **Done** (2026-10-31): 11a-11e. All test programs (programs.rs) + build.rs pass. Empty main 340 bytes (was ≈ 13.8 KB); see Results.

## Rules (agreed)

- **Piece:** code from one label up to the next label (each label starts a piece, `__L…` labels inside C functions too), or one `.data` or one `.bss` variable.
- **Uses:** every name a piece refers to: operands (`Address::Symbol`, `Value8::Msb/Lsb` in `Imm`, `Pair`, `Stack` offsets, `Mem`) and data values (`DataValue::Byte(Value8)`, `DataValue::Address`).
- **Fall-through:** a code piece whose last instruction isn't an unconditional jump (`jmp`, `ret`, `irt`; check the list in `hbc2_core::Opcode`; `hlt` does fall through) also uses the piece placed right after it in the same block. An empty piece (label directly followed by a label) falls through too. Pieces in hand-written asm need no markers.
- **Roots:** `_start`, every block or `.data` at a fixed address (the interrupt vectors at $0xFE00 in `runtime/devices.has`), names listed with the new `.keep name` directive (code reached only through a computed address).
- `__bss_start` / `__bss_end` are always defined (crt0 uses them); the `.bss` area holds only the kept variables.
- **Undefined names** are still reported in the whole program, removed pieces included, before removal: a typo in an unused function fails the build (like a C compiler).
- **On for C builds only.** Assembly projects keep everything (off by default in hbc2-asm).
- **Library names give way** (11d): when the program defines a name the library also defines, the library's piece is left out and every use binds to the program's.
- Known limit (to write in the manual): code that relies on two labels being next to each other in memory without naming the second one.

## Steps

- [x] **11a. Removal pass in hbc2-asm**
  - Options: `assemble(entry, provider, target)` stays; add an options form (e.g. `Options { target, strip: bool }` + `assemble_with`) so the IDE/CLI callers don't change.
  - In `assembler.rs`: between `group()` and `place()`, a `strip()` that splits blocks at labels into pieces, builds the use graph, walks from the roots, and rebuilds the blocks/data/bss with the kept pieces (kept pieces of one block stay together in order: offsets of the labels are recomputed). A fixed-address block is kept whole.
  - `.keep name` directive: parser (`Stmt::Keep`), error if the name is never defined.
  - Undefined-name check before stripping (see Rules).
  - Tests in `tests/assemble.rs`: unused label removed; fall-through kept (conditional jump at the end, empty piece); piece used only from `.data` kept; `.data` / `.bss` only used by removed code removed; vector table at a fixed address and its handler kept; `.keep`; `name.msb` use counts; `strip: false` = image unchanged; undefined name in dead code still an error. Mutation-check the walker.
- [x] **11b. Report**
  - `Program` gets what was removed (e.g. `removed: Vec<(name, file, line, size)>`, bytes saved).
  - CLI: `hbc2 cc … --map` prints kept/removed per name with sizes. Optional: the IDE build message says "N bytes (M removed)".
- [x] **11c. On for C builds**
  - `hbc2_cc::build_program` assembles with `strip: true` (the IDE and `hbc2 cc` go through it: no IDE change).
  - Before turning it on: record sizes of all test programs (programs.rs prints them) in "Results" below, then compare.
  - All 57 programs + `tests/build.rs` must pass. New size tests: empty `main` (expect only crt0 + devices + exit), hello-world with `puts`, one with `printf`. `__cc*` compare helpers must vanish everywhere.
  - Check `lines.rs` / IDE: a breakpoint on a C line whose code was removed must be refused cleanly (the line just has no address).
  - Check `set_vector` handlers (tests/multi/vector) stay.
- [x] **11d. Library names give way to the program's**
  - Directive `.library`: the names defined in the rest of the file and in the files it includes are library names. `lib/lib.has` and the generated `library.has` start with it (`build_library` writes it). Not the runtime (`__` names are reserved; `_start` must never be replaced).
  - In `group()`: a duplicate where one side is a library name → the library definition's piece is dropped, no error. Two library definitions, or two program definitions → error as today. If a kept library piece falls through into a dropped one → error naming both.
  - Library statics are already renamed per unit (`__S<unit>_…`), so they never clash.
  - Remove the "known: a program defining a library name is a duplicate" note (notes/multifile.md). Tests: a program with its own `puts` (library `puts` gone, `printf` still works if it calls `putchar` only); a `.has` file overriding `putchar` used by library `puts`.
- [x] **11e. Docs and wrap-up**
  - `Documentation/` assembly manual: `.keep`, `.library`, the removal rules and the known limit.
  - `lib/README.md` "Size", `ABI.md` if it mentions program size or linking, `ROADMAP.md` status/step 11, `notes/session-log.md`.
  - `cargo test --workspace`, clippy, fmt; commit.

## Later (not in step 11)

- **Per-project option for assembly projects** (user, 2026-10-31): a project setting to turn the removal on for asm projects (off by default: unused routines may be kept on purpose to call them from the debugger).
- Common symbols (tentative `int n;` in two files is a duplicate today).

## How it works

- hbc2-asm: `assemble_with(entry, provider, Options { target, strip })`; `assemble` = strip off. `Program.removed: Vec<Removed>` (name, size, kind, file, line; source order), `Program::bytes_removed()`.
- `assembler.rs`: `group()` → `strip()` (only with `strip`) → `place()`. `strip`: pieces (one per label, `.data`, `.bss`), uses from operands and data values, fall-through edge unless the piece ends with jmp/ret/irt; undefined names reported there (all pieces); roots = the `_start` piece, fixed blocks (whole) and fixed `.data`, `.keep`; blocks rebuilt with label offsets recomputed.
- **`_start` exception:** `_start` gets address $0x0000, so its block counts as fixed, but it is NOT kept whole (only the `_start` piece is a root). Without this, crt0.has's whole block (and anything after `_start` in a file) stayed.
- `.keep name`: `Stmt::Keep` (parser); undefined → error even without strip (in `group`).
- `.library`: handled in `preprocess.rs` (a flag per line, inherited by included files, not passed back to the including file); `Item::Stmt(stmt, library)`. `group()` → `Names::define`: a duplicate with exactly one library side → the library definition is renamed `"<name> (library, replaced)"` (a space: no source can refer to it), in place if it came first (`At` = where each definition is). Strip then drops it.
- **Change from the plan:** a kept library piece falling through into a replaced one is NOT an error: the fall-through keeps the library's own code (hidden name), so the library behaves as before; only the name binds to the program's. Written in the manual.
- hbc2-cc: `build_program` strips; `build_library` writes `.library` first in `library.has`; `lib/lib.has` starts with `.library`; `lines.rs` remaps `removed` to C lines too. CLI: `hbc2 cc --map`; the build line says "N bytes left out". IDE build message: "(N unused bytes left out)". Breakpoints on removed C lines: no address → skipped like comment lines (nothing to change).
- Tests: hbc2-asm `tests/assemble.rs` `strip_*`, `library_*` (walker mutation-checked: fall-through, ends_flow, uses, roots, keeps, msb/lsb, data values); hbc2-cc `tests/build.rs` `sizes` (empty / hello / printf / unused function + statics, no `__cc*` kept) and `undefined_in_unused_function`; `tests/multi/override` (own `puts` in C, own `putchar` in .has used by the library's printf; mutation-checked). programs.rs prints "bytes in all" and no longer needs `term_kbd_ready` (left out when the keyboard isn't used).
- Shell note for later sessions: the Bash tool mangles backslashes and some quotes in heredocs; write edit scripts to the scratchpad with Write and run them.

## Results

Whole program (`bytes_used`: code + data, library + runtime), before / after turning strip on. multi/override is new (no "before").

| Program | Before | After |
|---|---|---|
| basics.c | 14767 | 1305 |
| bench.c | 18134 | 5104 |
| frames.c | 14162 | 700 |
| hardware.c | 16160 | 4809 |
| peephole.c | 15638 | 2648 |
| printf.c | 15045 | 6011 |
| stdio.c | 14836 | 4248 |
| strings.c | 18599 | 7809 |
| ops/char_assign_g.c | 22450 | 9628 |
| ops/char_assign_l.c | 22766 | 9944 |
| ops/char_cond_g.c | 28726 | 15264 |
| ops/char_cond_l.c | 28762 | 15300 |
| ops/char_const_g.c | 20746 | 7592 |
| ops/char_const_l.c | 20738 | 7584 |
| ops/char_ops_g.c | 19958 | 6972 |
| ops/char_ops_l.c | 19954 | 6968 |
| ops/int_assign_g.c | 26150 | 13556 |
| ops/int_assign_l.c | 26558 | 13964 |
| ops/int_cond_g.c | 33134 | 19672 |
| ops/int_cond_l.c | 33130 | 19668 |
| ops/int_const_g.c | 21282 | 8128 |
| ops/int_const_l.c | 21234 | 8080 |
| ops/int_ops_g.c | 21466 | 8480 |
| ops/int_ops_l.c | 21322 | 8336 |
| ops/long_assign_g.c | 34230 | 22232 |
| ops/long_assign_l.c | 34378 | 22380 |
| ops/long_cond_g.c | 40538 | 27076 |
| ops/long_cond_l.c | 40394 | 26932 |
| ops/long_const_g.c | 29190 | 16568 |
| ops/long_const_l.c | 29398 | 16776 |
| ops/long_ops_g.c | 26870 | 14248 |
| ops/long_ops_l.c | 26882 | 14260 |
| ops/signed_char_assign_g.c | 22586 | 10000 |
| ops/signed_char_assign_l.c | 22806 | 10220 |
| ops/signed_char_cond_g.c | 29582 | 16120 |
| ops/signed_char_cond_l.c | 29534 | 16072 |
| ops/signed_char_const_g.c | 21982 | 8828 |
| ops/signed_char_const_l.c | 21994 | 8840 |
| ops/signed_char_ops_g.c | 21502 | 8516 |
| ops/signed_char_ops_l.c | 21454 | 8468 |
| ops/unsigned_assign_g.c | 26074 | 13324 |
| ops/unsigned_assign_l.c | 26554 | 13804 |
| ops/unsigned_cond_g.c | 32414 | 18952 |
| ops/unsigned_cond_l.c | 32374 | 18912 |
| ops/unsigned_const_g.c | 21218 | 7824 |
| ops/unsigned_const_l.c | 21222 | 7828 |
| ops/unsigned_long_assign_g.c | 34202 | 22008 |
| ops/unsigned_long_assign_l.c | 34302 | 22108 |
| ops/unsigned_long_cond_g.c | 39842 | 26380 |
| ops/unsigned_long_cond_l.c | 39670 | 26208 |
| ops/unsigned_long_const_g.c | 28350 | 15492 |
| ops/unsigned_long_const_l.c | 28642 | 15784 |
| ops/unsigned_long_ops_g.c | 26638 | 13784 |
| ops/unsigned_long_ops_l.c | 26514 | 13660 |
| ops/unsigned_ops_g.c | 21294 | 8152 |
| ops/unsigned_ops_l.c | 21230 | 8088 |
| multi/files/main.c | 14760 | 1558 |
| multi/override/main.c | — | 4114 |
| multi/vector/main.c | 14086 | 705 |
