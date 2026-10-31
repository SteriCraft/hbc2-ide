# HBC-2 C compiler — roadmap (Claude's tracking file)

C compiler for the HBC-2 based on the Fuzix Compiler Kit (FCC):
`cc0` (tokens) → `cc1` (parse, trees) → `cc2` (our backend, prints `.has`) → `copt` (peephole) → hbc2-asm.
The user only says "go to the roadmap": read this file, continue at **Next**, update the status here at the end of each session.

## Status

- **Phase:** steps 1-12 done (step 11 = linker, details in `LINKER.md`; step 12 = backend speed-ups + string.h in asm, 2026-10-31). 62 test programs (programs.rs, multi-file ones included) pass + `tests/build.rs`. Tests: `cargo test -p hbc2-cc --release` (needs `compiler/build.cmd` first). Unreached code is left out: an empty `main` is 340 bytes, hello world 1.7 KB, with printf 4.8 KB (lib/README.md "Size").
- **Next:** nothing planned. Ask the user what comes next; candidates: EEPROM, float, the backend ideas below. The IDE builds through `hbc2_cc::build_program`; keep that the one entry point.
- Possible later backend work: carry what the registers hold across loop labels (a dataflow version of reloads.rs; looked at bench.c's loops: the paths into a loop label rarely agree, small gain expected); `x *= k` / `x /= k` in place like `<<=`.

## Notes files (read only when needed)

Details moved out of this file to keep it short. Open the one for the part you touch:

| File | Content |
|---|---|
| `notes/pipeline.md` | FCC tool chain, command lines, cc1 stdout quirk |
| `notes/step1-design.md` | data/labels/statics/arguments as FCC emits them |
| `notes/backend.md` | backend-hbc2.c: rewrites, labels, data buffering, exports |
| `notes/inline.md` | gen_shortcut inline code, conditions on flags, measured gains |
| `notes/peephole.md` | copt and rules.hbc2 |
| `notes/helpers.md` | the 89 runtime helper names |
| `notes/library.md` | compiler/lib layout, device detection, terminal from C |
| `notes/multifile.md` | build_program, static renaming, error messages |
| `notes/ide.md` | diagnostics, line markers, asm→C line map, line step, background build |
| `notes/misc.md` | CPU facts, FCC quirks, stack/frame tricks, math library location |
| `notes/reloads.md` | step 12: reload removal pass (reloads.rs), volatile markers |
| `notes/session-log.md` | what each session did |

## Decisions (agreed 2026-10-30)

1. **Linking:** (a) the driver joins every generated `.has` + the runtime into one program.
   (b): unused code dropped by hbc2-asm: step 11, decision 15 (done).
2. **Names:**
   - C `foo` → `_foo` (so asm `screen_putc` and C wrapper `_screen_putc` coexist).
   - Runtime helpers: `__name` (Fuzix's names where they exist: `__mul`, `__cceq`…).
   - `static` names: per-file prefix `__S<n>_name` (n = file number in the build; was `_S<n>_` until step 9: `__` can't clash with a C name, like `__L`).
   - Compiler labels: `__L<n>_…` with the file number (changed from `_L<n>` in step 1: a C name `L1` would give `_L1`; C reserves `__` names, so `__L` can't clash).
3. **Registers:**
   - `[a:b]` = working value (16-bit, a = MSB); `char` in `b`.
   - `[x:y]` = pointer register (`*p`, `p->f` → `[x:y + n]`).
   - `[c:d]` = second operand / high half of a `long` (`long` = `[c:d]:[a:b]`).
   - `i`, `j` = scratch / second pointer.
   - Matches the math library's 16-bit routines ([a:b] op1/result, [c:d] op2). 32-bit routines use pointers ([x:y], [i:j]) → need adapters (see open questions).
4. **Calls:**
   - Arguments pushed right to left, each value low byte first (big-endian in memory, like `cal`).
   - Caller removes them (`asp n`) → variadic functions work.
   - Return in `[a:b]`, `long` in `[c:d]:[a:b]`.
   - Every register except SP may be changed by a function (caller-saved). No register variables: dropped (user, 2026-10-31); `register` stays accepted and ignored.
5. **Asm library from C:** compiler helpers get their own entry points matching what the backend emits (built from the math library code); user-facing drivers (screen, terminal, keyboard) keep their register interface and get thin C wrappers.
6. **Types:** `char` unsigned by default (already FCC's default: `__castuc_`). `int`/`short` 16, `long` 32, pointers 16. No `float` for now: the compiler must STOP with a clear error at the C line (plan: `error()` in `target_sizeof` of `target-hbc2.c` for FLOAT/DOUBLE).
7. **Frames > 255 bytes:** compute the address with `rsp` (see notes), no compile error.
8. **EEPROM:** not handled now; the IDE loads the program into RAM. The user writes custom boot code later (C or asm) if needed.
9. **Interrupt handlers:** hand-written asm only (they may call C functions). No compiler support.
10. **Zeroed variables (BSS):** `.bss name size` in hbc2-asm (DONE 2026-10-30): all `.bss` placed together after code and data, nothing in the image, `__bss_start` / `__bss_end` defined; `crt0` clears the area (loop example in the assembly manual).
11. **Licence/packaging:** not a concern (single user). Repo is GPLv3 like the kit.
12. **Library location (step 8):** `compiler/lib/` holds COPIES of the user's drivers (monitor driver, terminal) + the C library (`include/` headers, sources). The user's originals in design_projects stay untouched.
13. **Device detection (step 8):** `crt0` handles it: installs the port-0 interrupt handler (copy of the user's interruptHandler.has logic) and waits for the devices before calling `main`, so C code can print straight away.
14. **printf (step 8):** minimal set: `%d %u %x %c %s %%`, `l` for long, width + `0` padding. No float.
16. **IDE integration (user, 2026-10-31, step 10):** a C project = a folder with `main.c` (wins over `main.has`); every `.c` and `.has` file of the folder is built (main.c first = unit 1, then name order); Build (F7) saves all open files first; tools stay in the repo's `compiler/bin` (`HBC2_COMPILER_DIR` overrides); an asm project cannot call C files; no interrupt handler template for C (crt0 + set_vector are enough); the asm→C line map is part of step 10.
15. **Linker (user, 2026-10-31; replaces the object-file plan):** no object files. hbc2-asm drops every piece of the program that can't be reached from `_start` / fixed addresses / `.keep` (piece = from a label to the next label, or one `.data`/`.bss`; fall-through counts as a use). On for C builds only; assembly projects keep everything (a per-project option may come later). Library names give way to the program's own (a program may define `puts`). Plan and progress: `LINKER.md`.

## Open questions (waiting for the user)

- (none)

## Layout

- `compiler/fcc/` — copy of the kit's core (see `fcc/UPSTREAM.md`: commit, licences, what was left out). HBC-2 files to add: `target-hbc2.c` (cc1), `backend-hbc2.c` (cc2). Keep upstream files unchanged where possible; list any change in UPSTREAM.md.
- `compiler/msvc/` — `compat.h` (forced include: io.h names, binary `open`, `itoa`/`_itoa` renamed, strcasecmp, strlcpy/strlcat), `compat.c` (binary stdin/stdout/stderr before main via `.CRT$XCU`), `unistd.h` stub.
- `compiler/build.cmd` — MSVC build (vcvars64 of VS 2022 Community) → `compiler/bin/*.exe`, objects/logs in `compiler/build/` (both git-ignored). Builds cc0, cpp, copt, cc1.ref (target-6800.c), cc2.ref (backend-default.c). Add cc1.hbc2/cc2.hbc2 in step 3.
- Full upstream clone for reading other backends: re-clone https://github.com/EtchedPixels/Fuzix-Compiler-Kit at the commit in UPSTREAM.md (scratchpad copies don't last). 6809 = `be-codegen-6800.c` + `be-code-6809.c` + `be-func-6800.c` + `be-track-6800.c`; cc1 for 6809 = `target-6800.c`.
- Possible later (step 10): build the C tools from cargo (`cc` crate, one tiny bin crate per tool) so `cargo build` does everything.

## Steps

- [x] 1. Get FCC, build it, compile tests for the 6809 target, read the backends. (No WSL/MSYS2/gcc on this machine: built with MSVC + compat layer instead. 6809 output checked on a loop + call test.)
- [x] 2. Write the ABI document — `compiler/ABI.md`. Section 11 (helpers) is a draft to settle in steps 3-4.
- [x] 3. Minimal backend `backend-hbc2.c`: constants, load/store globals + locals, calls, prologue/epilogue, labels, jumps, branches; everything else → helpers. Also inline already: + - & | ^ ~ unary- (8/16), == != (8/16), ! and bool, casts (incl. sign extension), deref / assign through pointers, frames > 255.
- [x] 4. Runtime: `crt0.has` + 89 helpers (`runtime.has` includes compare, long, mul, div, shift, assign, switch; ≈3.9 KB). Written for the C convention, NOT wrappers of the user's math library (it saves all registers; runtime must be self-contained).
- [x] 5. Test harness: crate `hbc2/crates/hbc2-cc` (lib: `compile()` = Rust version of the driver; tests/helpers.rs: each helper called directly on the emulator vs a Rust model, breakpoint on a return label; tests/programs.rs: every `compiler/tests/*.c`, "Expected: main returns N" on line 1). Both checked by mutation (a broken helper makes them fail). FCC's own test programs: not run yet (upstream `test/` not copied).
- [x] 6. Inline code (see "Inline code (step 6)" below).
- [x] 7. `copt` peephole rules (see "Peephole rules (step 7)" below).
- [x] 8. Library: `hbc2.h` (monitor/keyboard/RTC wrappers, in/out), `putchar`/`puts` on `term_putc`, `string.h`, small `printf`.
  - [x] 8a. `hbc2.h` + `asm/hbc2.has` wrappers, driver copies in `lib/asm`, device detection in crt0 (`runtime/devices.has`), `exit` (stdlib.h), `-I lib/include` in both drivers, `build_library()`, tests/hardware.c (+ `.screen` / `.keys` support in programs.rs, all tests now run with monitor + RTC + keyboard).
  - [x] 8b. stdio.h: `putchar`/`puts` in asm (`asm/stdio.has`, first output clears the screen: `lib_term_start`), `getchar`/`gets` in C (`src/stdio.c`, line-buffered on term_readline). tests/stdio.c.
  - [x] 8c. string.h (14 functions, in C: `src/string.c`; in asm since step 12: `asm/string.has`) + stddef.h. tests/strings.c (mutation-checked).
  - [x] 8d. printf (decision 14, `src/printf.c`) + stdarg.h. tests/printf.c (mutation-checked).
- [x] 9. Multi-file builds: `hbc2_cc::build_program(sources, library, dir)` (`.c` + `.has`), static renaming in `compile()` (both drivers), `-I` source folder, C-level error messages, library statics (`next`, `field`). Tests: `tests/multi/files`, `tests/build.rs` (mutation-checked).
- [x] 10. IDE (decision 16, see "IDE integration (step 10)" below):
  - [x] 10a. `hbc2 cc <files…> [-o out.bin] [--dir build] [--symbols]`; `hbc2 run main.c …` builds first.
  - [x] 10b. Diagnostics instead of strings (`compile`, `build_library`, `build_program` → `Vec<Diagnostic>`, `report()` for text); every C file compiled even when one fails; library files kept when newer than their inputs.
  - [x] 10c. C projects in the IDE (project language, template `hbc2/templates/main.c`, Build saves then compiles into `<project>/.build`, RAM only).
  - [x] 10d. C highlighting; compiled files and the compiler's own files open read-only.
  - [x] 10e. asm→C line map (`;#line n` markers through FCC, `lines.rs` remaps the Program).
- [x] 12. Speed-ups (user, 2026-10-31): one-byte bit tests (`x & 0x8000`) in conditions / ?: / 0-1 values; `<<=` `>>=` by a constant in place; reload removal after copt (`hbc2-cc/src/reloads.rs`, volatile-aware; copt's store-then-load rules removed); string.h in asm (`lib/asm/string.has`, 2.7 KB → 1.1 KB). tests/bits.c, tests/reloads.c, build.rs volatile_reloads; all mutation-checked. See notes/reloads.md.
- [x] 11. Linker (decision 15) — see `LINKER.md`: `Options::strip` in hbc2-asm (on for C builds), `.keep`, `.library` (program names replace the library's), `hbc2 cc --map`.

Later: EEPROM, float.

