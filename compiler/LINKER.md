# Step 11: linker (Claude's tracking file)

Goal: a C program carries only the code and data it can reach (library, runtime helpers, its own files). Today every program carries ≈ 13.2 KB (library 9.3 KB + runtime 3.9 KB).
Decision 15 in `ROADMAP.md`: no object files. hbc2-asm removes what can't be reached. Agreed with the user on 2026-10-31.
Continue at **Next**, tick the boxes, and add notes under "How it works" as the work goes (keep them short).

## Status

- **Next:** 11a. Nothing written yet.

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

- [ ] **11a. Removal pass in hbc2-asm**
  - Options: `assemble(entry, provider, target)` stays; add an options form (e.g. `Options { target, strip: bool }` + `assemble_with`) so the IDE/CLI callers don't change.
  - In `assembler.rs`: between `group()` and `place()`, a `strip()` that splits blocks at labels into pieces, builds the use graph, walks from the roots, and rebuilds the blocks/data/bss with the kept pieces (kept pieces of one block stay together in order: offsets of the labels are recomputed). A fixed-address block is kept whole.
  - `.keep name` directive: parser (`Stmt::Keep`), error if the name is never defined.
  - Undefined-name check before stripping (see Rules).
  - Tests in `tests/assemble.rs`: unused label removed; fall-through kept (conditional jump at the end, empty piece); piece used only from `.data` kept; `.data` / `.bss` only used by removed code removed; vector table at a fixed address and its handler kept; `.keep`; `name.msb` use counts; `strip: false` = image unchanged; undefined name in dead code still an error. Mutation-check the walker.
- [ ] **11b. Report**
  - `Program` gets what was removed (e.g. `removed: Vec<(name, file, line, size)>`, bytes saved).
  - CLI: `hbc2 cc … --map` prints kept/removed per name with sizes. Optional: the IDE build message says "N bytes (M removed)".
- [ ] **11c. On for C builds**
  - `hbc2_cc::build_program` assembles with `strip: true` (the IDE and `hbc2 cc` go through it: no IDE change).
  - Before turning it on: record sizes of all test programs (programs.rs prints them) in "Results" below, then compare.
  - All 57 programs + `tests/build.rs` must pass. New size tests: empty `main` (expect only crt0 + devices + exit), hello-world with `puts`, one with `printf`. `__cc*` compare helpers must vanish everywhere.
  - Check `lines.rs` / IDE: a breakpoint on a C line whose code was removed must be refused cleanly (the line just has no address).
  - Check `set_vector` handlers (tests/multi/vector) stay.
- [ ] **11d. Library names give way to the program's**
  - Directive `.library`: the names defined in the rest of the file and in the files it includes are library names. `lib/lib.has` and the generated `library.has` start with it (`build_library` writes it). Not the runtime (`__` names are reserved; `_start` must never be replaced).
  - In `group()`: a duplicate where one side is a library name → the library definition's piece is dropped, no error. Two library definitions, or two program definitions → error as today. If a kept library piece falls through into a dropped one → error naming both.
  - Library statics are already renamed per unit (`__S<unit>_…`), so they never clash.
  - Remove the "known: a program defining a library name is a duplicate" note (notes/multifile.md). Tests: a program with its own `puts` (library `puts` gone, `printf` still works if it calls `putchar` only); a `.has` file overriding `putchar` used by library `puts`.
- [ ] **11e. Docs and wrap-up**
  - `Documentation/` assembly manual: `.keep`, `.library`, the removal rules and the known limit.
  - `lib/README.md` "Size", `ABI.md` if it mentions program size or linking, `ROADMAP.md` status/step 11, `notes/session-log.md`.
  - `cargo test --workspace`, clippy, fmt; commit.

## Later (not in step 11)

- **Per-project option for assembly projects** (user, 2026-10-31): a project setting to turn the removal on for asm projects (off by default: unused routines may be kept on purpose to call them from the debugger).
- Common symbols (tentative `int n;` in two files is a duplicate today).

## How it works

(fill in as the work goes)

## Results

(sizes before / after, per test program)
