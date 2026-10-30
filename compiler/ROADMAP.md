# HBC-2 C compiler — roadmap (Claude's tracking file)

C compiler for the HBC-2 based on the Fuzix Compiler Kit (FCC):
`cc0` (tokens) → `cc1` (parse, trees) → `cc2` (our backend, prints `.has`) → `copt` (peephole) → hbc2-asm.
The user only says "go to the roadmap": read this file, continue at **Next**, update the status here at the end of each session.

## Status

- **Phase:** step 10 done (IDE integration, NOT committed yet: the user decides). Steps 8-9 committed (5e182ec, eadfaee). Before step 10: 57 C programs pass (56 single files + `tests/multi/files`), plus `tests/build.rs` (builds that must fail). Step 8: 4 library tests. Library ≈ 9.3 KB in every program (lib/README.md "Size"): string.c 2.7 KB + printf.c 2.5 KB compiled; rewriting string.h in asm would save ≈ 1.8 KB (offered to the user). Before step 8: 52 C programs pass (`tests/*.c` + 48 generated `tests/ops/*.c`, ≈12 000 checks). bench.c: 4464 → 4308 bytes, 253 253 → 244 971 ticks with copt (all tests: −9.2 % code, −3.2 % ticks). Tests: `cargo test -p hbc2-cc --release` (needs `compiler/build.cmd` first). Steps 1-7 committed (step 7: 3c39796).
- **Next:** step 11 linker. After step 10 the user asked for (done, same session): F10 steps a whole C line (rule: until the source place changes; into a .has file = an ordinary instruction step), and C builds in the background. The IDE builds through `hbc2_cc::build_program`; keep that the one entry point so the linker fits behind it later without IDE changes. No interim size filter (user: keep it simple until the linker). Possible later backend work (bigger gains than copt, need knowledge copt lacks): keep track of what [a:b] / [x:y] hold to drop reloads across statements; `x << 1` assignment forms inline (`crc <<= 1` calls `__shleq`); `x & 0x8000` tests the top byte only.

## Decisions (agreed 2026-10-30)

1. **Linking:** (a) the driver joins every generated `.has` + the runtime into one program.
   (b): object files + linker in hbc2-asm, to drop unused code (memory usage): step 11, decision 15.
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
15. **Linker (later, user asked 2026-10-31):** object files + linker in hbc2-asm, the C library precompiled once into an archive; only the functions a program reaches are kept (function-level: each function its own section). Applies to the runtime helpers too (≈3.9 KB in every program today). Memory is tight: this matters.

## Open questions (waiting for the user)

- (none)

## Layout

- `compiler/fcc/` — copy of the kit's core (see `fcc/UPSTREAM.md`: commit, licences, what was left out). HBC-2 files to add: `target-hbc2.c` (cc1), `backend-hbc2.c` (cc2). Keep upstream files unchanged where possible; list any change in UPSTREAM.md.
- `compiler/msvc/` — `compat.h` (forced include: io.h names, binary `open`, `itoa`/`_itoa` renamed, strcasecmp, strlcpy/strlcat), `compat.c` (binary stdin/stdout/stderr before main via `.CRT$XCU`), `unistd.h` stub.
- `compiler/build.cmd` — MSVC build (vcvars64 of VS 2022 Community) → `compiler/bin/*.exe`, objects/logs in `compiler/build/` (both git-ignored). Builds cc0, cpp, copt, cc1.ref (target-6800.c), cc2.ref (backend-default.c). Add cc1.hbc2/cc2.hbc2 in step 3.
- Full upstream clone for reading other backends: re-clone https://github.com/EtchedPixels/Fuzix-Compiler-Kit at the commit in UPSTREAM.md (scratchpad copies don't last). 6809 = `be-codegen-6800.c` + `be-code-6809.c` + `be-func-6800.c` + `be-track-6800.c`; cc1 for 6809 = `target-6800.c`.
- Possible later (step 10): build the C tools from cargo (`cc` crate, one tiny bin crate per tool) so `cargo build` does everything.

## Pipeline (from upstream cc.c)

`cpp -E [-I…] [-D…] f.c > f.%` → `cc0 f.sym < f.% > f.@` → `cc1 <cpucode> <features> < f.@ > f.#` → `cc2 f.sym <cpucode> <opt 0/1/2/s> <features> [codeseg] < f.# > f.^` → `copt rules < f.^ > f.s` (with opt 0: cc2 writes f.s, no copt).
- cc1 reads back its own stdout (lseek/read on fd 1): the output file must be opened read+write (Rust: `OpenOptions::new().read(true).write(true).create(true).truncate(true)`).
- Trial driver used in step 1: python, `subprocess.run(..., stdin=open(in,'rb'), stdout=open(out,'w+b'))`.

## Design points found in step 1 (for the ABI / backend)

- **Data:** FCC emits a data label then one value at a time (`gen_value` per byte/word/long, `gen_name` = address `name+off`, `gen_text_data` = string literal address, `gen_space` = zeros). hbc2-asm needs ONE `.data` line per object (floating `.data` lines are not guaranteed contiguous). → backend buffers values and prints `.data _name v, v, …` when the object ends. Words → two bytes; `name+off` addresses need a byte form: `.data` only takes `name.msb/.lsb` WITHOUT offset → small assembler addition needed (e.g. allow `name + n` addresses inside `.data` as 2 bytes, or `(name + n).msb`). Decide in step 3 (assembler convenience, not ISA).
- **BSS segment** → `.bss _name size` (gen_space in A_BSS).
- **Labels:** FCC labels `L<n><tail>` (tails `_r` return, `_l` loop, `_b` break, `_c` continue, `_n` …), literals `T<n>`, switch tables `Sw<n>`; numbers restart in every file → print `__L<unit>_<n><tail>`, `__T<unit>_<n>`, `__W<unit>_<n>`; unit number given to cc2 as an extra argument (argv[5], the "codeseg" slot, is free for our backend).
- **Statics:** cc2 prints `_s` for statics, just without `.export` (references don't say static/global). → backend prints a marker for exported names (e.g. `; export _g`) and the DRIVER renames non-exported `_name` symbols of that file to `__S<unit>_name` (done, step 9).
- **Arguments:** FCC's T_ARGUMENT offsets start at 0 for the first argument; `target_argsize` = real size (chars passed as 1 byte — fits `psh`).
- **Helpers:** default backend emits `call __enter`, `__loadl`, `__deref`, `__plus`, `__castuc_`, `__cleanup`… → names `__<op><type suffix>`. Our gen_helpcall prints `cal __`.

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
  - [x] 8c. string.h (14 functions, in C: `src/string.c`) + stddef.h. tests/strings.c (mutation-checked). Possible later: memcpy/memset in asm if they show up as slow.
  - [x] 8d. printf (decision 14, `src/printf.c`) + stdarg.h. tests/printf.c (mutation-checked).
- [x] 9. Multi-file builds: `hbc2_cc::build_program(sources, library, dir)` (`.c` + `.has`), static renaming in `compile()` (both drivers), `-I` source folder, C-level error messages, library statics (`next`, `field`). Tests: `tests/multi/files`, `tests/build.rs` (mutation-checked).
- [x] 10. IDE (decision 16, see "IDE integration (step 10)" below):
  - [x] 10a. `hbc2 cc <files…> [-o out.bin] [--dir build] [--symbols]`; `hbc2 run main.c …` builds first.
  - [x] 10b. Diagnostics instead of strings (`compile`, `build_library`, `build_program` → `Vec<Diagnostic>`, `report()` for text); every C file compiled even when one fails; library files kept when newer than their inputs.
  - [x] 10c. C projects in the IDE (project language, template `hbc2/templates/main.c`, Build saves then compiles into `<project>/.build`, RAM only).
  - [x] 10d. C highlighting; compiled files and the compiler's own files open read-only.
  - [x] 10e. asm→C line map (`;#line n` markers through FCC, `lines.rs` remaps the Program).
- [ ] 11. Linker (decision 15).

Later: EEPROM, float.

## Backend (step 3) — how it works

- Rewrites (gen_rewrite_node): casts that change nothing removed; FUNCCALL(NAME) → T_CALLNAME; NAME/LOCAL/ARGUMENT/LABEL + constant merged; DEREF/EQ of LOCAL/ARGUMENT → T_LREF/T_LSTORE (argument offsets += argbase 2 + frame_len), of NAME → T_NREF/T_NSTORE, of LABEL → T_LBREF/T_LBSTORE (sizes 1, 2, 4).
- `sp` = bytes pushed since the frame; stack offsets = node value + sp. `work_size` = size of the last value, used by jfalse/jtrue (`or b, 0` / `mov i, a; or i, b` …; jtrue needs a jump over a jmp: no "jump if not zero" instruction).
- Labels: `__L<unit>_<n><tail>` (FCC's), `__X<unit>_<n>` (backend's own), `__T<unit>_<n>` literals and local statics, `__W<unit>_<n>` switch tables, `__W<unit>_<tag>_<entry>` case labels. Unit = cc2 argv[5] (default 1).
- Data: one pending object per area (A_DATA, A_LITERAL, A_BSS): FCC can start a string literal in the middle of another object (`char *s = "hi"`). Flushed when the code area comes back, when a new object starts in the same area, at the end. char values: printable runs as "…"; wider values as 0xNN bytes.
- `; export _name` comment lines: used by the static renaming (see "Multi-file builds").
- Helper calls: gen_helpcall pops the left operand (pop_left, rules in ABI.md 11), then `cal __`.
- Temporary driver: `compiler/tools/hbc2cc.py file.c -o file.has [--unit N] [--no-copt]` (cpp → cc0 → cc1.hbc2 → cc2.hbc2 → copt; cc2 opt level 0: our backend ignores it).
- Known: cc1 reports the float error at the wrong line (end of the declarations, e.g. line 3 for line 1).

## Inline code (step 6) — how it works

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

## Peephole rules (step 7) — how it works

- `compiler/fcc/rules.hbc2`, run by `copt` (bin/copt.exe, Fraser's, from Z88DK) after cc2, in both drivers (`hbc2_cc::compile`, hbc2cc.py). The file's header explains the format and what the rules rely on.
- copt matches backwards from each line, in rule order; `%n` = text up to the next pattern char; `%eval R = rpn` / `%check lo <= %n <= hi` are conditions (put them first: checked after the lines match); `%eval(rpn)` in a replacement; `%activate` makes rules at run time (used for jump threading: `L: jmp M` → every `jm? L` becomes `jm? M`; `L: ret` → `jmp L` becomes `ret`). Only `%activate` starts another pass (max 16); a rule fires at most 65535 times, which also ends jump cycles (`a: goto b; b: goto a;` checked: fast, code right).
- Rules: for-loop `jm? __X / jmp B / __X: jmp N` → `jm? N / jmp B`; jump to the next line; threading; tail call `cal f / L_r: / ret` → `jmp f`; dead code after jmp; store then load of the same place (long / int / byte; the int and long forms check that the places differ); `psh b, a / 2 loads / pop d, c` → `mov c, a / mov d, b / loads` (stack offsets −2); redundant movs.
- Coverage: each rule was removed in turn and the 52 outputs compared; every rule changes at least one (except the `mov r, r` guard). Mutation check: 6 broken rules, each fails ≥ 1 program (peephole.c's `longs` was extended for this).
- programs.rs now counts `psh`/`pop` lists as one instruction per register (the numbers above use this count).
- Tried and left out (never fired in 52 programs): jump to a label after other labels, `jm? L / jmp L`, dead code after `ret`, `cal f / ret` without a label.

## Library (step 8) — how it works

- `compiler/lib`: `include/` (cpp gets `-I` in both drivers), `lib.has` (asm part), `src/*.c` compiled by `hbc2_cc::build_library(dir)` with units 100+ → `dir/library.has` (includes lib.has + compiled files). Programs: crt0, runtime, library.has, own files. See lib/README.md.
- `asm/screen.has`, `asm/terminal.has`: copies of the user's drivers, only the port names changed (`_monitor_port`…, header lists changes). Re-copy if the user changes the originals.
- Device detection: `runtime/devices.has` (copy of the template's port-0 handler, vector `.data` at $0xFE00, variables in `.data` not `.bss` because interrupts come before the .bss clear), crt0 waits 32 loop turns (1 was enough: power-on interrupts are taken before crt0's first instructions finish).
- Terminal from C: `lib_term_start` (hbc2.has) = the first output switches to text mode and clears (no prompt, unlike term_init; no wait for the monitor: without one output does nothing). putchar/puts in asm, getchar/gets in C (static `next`), printf in C (static `field`).
- Tests: programs.rs runs every program with monitor + RTC + keyboard (`MachineConfig::default()`), `name.screen` = expected text screen, `name.keys` = typed once `term_kbd_ready` = 1, one key per 20 000 ticks.

## Multi-file builds (step 9) — how it works

- `build_program(sources, &Library, dir) -> Build { program, compiled }`: `.c` compiled to `dir/<stem>_<unit>.has` (units 1, 2, … in list order), `.has` included as is; `dir/main.has` = crt0, runtime, library.has, sources; assembled (Target::Ram). `build_library` now returns `Library { path, files: (compiled, source) }`.
- Static renaming at the end of `compile()` (`rename_statics`; same code in hbc2cc.py): names defined (`_x:` label, `.data _x`, `.bss _x`, single `_`) without a `; export _x` line → `__S<unit>_x` on every identifier token, not in quotes/comments. Local statics are already `__T<unit>_n`.
- cpp gets `-I<source folder>` first (cpp finds the folder itself by the last `/`, which fails on the Rust driver's mixed `\`/`/` paths).
- Errors: hbc2-asm "already defined" now names the other file when it differs (`(file, line n)`); `c_message` swaps compiled paths for C files (drops the meaningless line), `'_x'`/`'__S2_x'` → `'x'`, "unknown label or variable" → "not defined:".
- Known: a program defining a library name (`puts`) is a duplicate until the linker; tentative `int n;` in two files is a duplicate (no common symbols); a `static` declared, never defined, but used would bind to another file's global (not checked).
- programs.rs: every folder in `tests/multi` is one program (sorted `.c`/`.has`, `main.c` holds "Expected", `main.screen`/`main.keys`).

## IDE integration (step 10) — how it works

- **Driver (hbc2-cc):** `messages.rs` parses stage stderr: cc0/cc1 `name.c:12 - msg` (name only: looked up next to the C file, then in lib/include), cpp `[ 'what' - ]file:12:error: msg` (cpp's line is one too high: −1; path normalized, cpp joins -I and name with '/'). Unknown lines → the C file, line 0. Stage succeeded + stderr = warnings. `BUILD_DIR` = ".build". `build_library` recompiles a file only when older than its .c, lib/include/*.h, bin/*.exe, rules.hbc2 or the running program (tests share one library build: OnceLock, parallel tests raced on the same folder).
- **Line markers:** cc1 writes `H_LINE` (data = line, 0 = another file than the C file) before each statement, local declaration, top-level declaration, the `}` of a function, and line 0 before `write_bss` (tentative `int n;` come at the end). cc0 sends the C file's name at once (a file starting with `#include` used to give the header's name first). cc2 `gen_line` flushes pending data, then prints `;#line n` (repeats skipped). copt skips markers when matching and keeps the ones inside replaced lines after the new lines. Check done: with the markers removed, all 62 compiled files are the same as before (3 only reordered: data lines placed at their own line). All FCC changes listed in fcc/UPSTREAM.md.
- **Remap (`lines.rs`):** after assembling, `LineMaps::apply` points every instruction and label of a compiled file at its C file + line (C files appended to `program.files`); lines with no marker (startup code, runtime, header code, tentative variables) keep their asm location. Assembler errors in compiled files get the C line too ("not defined: 'f'" on the calling line, "(a.c, line 2)" in "already defined"). Known: a line whose code copt removed entirely (e.g. `return g;` after `g = …;`) has no instruction, so no breakpoint there; a function without a frame has no instruction on its declaration line.
- **Line step / background build (after step 10, user's rules):** runner command `step_within(addresses)` (one instruction, then runs while PC is in the sorted set, at the current speed, state Running so Pause works; Run/Pause/Load/PowerCycle clear it; breakpoints not checked). IDE `State::c_line_addresses(pc)`: all RAM addresses of the PC's C line when its file is .c/.h, else None → `step(1)`. C builds: `assemble()` saves, spawns a thread (`build_c`), `State::building: PendingBuild` (receiver, edits at start, `then` = Run/Step pressed meanwhile); `poll_build` each frame finishes it (`finish_build`, shared with asm). Assembly projects still assemble on the UI thread (fast, and they use unsaved buffers). Tests: runner `step_within_runs_until_the_pc_leaves_the_addresses`, smoke `c_project_builds_and_runs` (steps 6 → 7 → into add → 7 → 8 → into puts (stdio.has) → one instruction; RTC off in that test: its interrupt could land a step in the handler).
- **IDE:** `Project.language` (main.c → C), `c_sources()`, `build_dir()`; `State::target()` (RAM for C), `State::is_generated()` (read-only: `.build/` and compiler_dir); `State::assemble()` builds C projects via `build_c`; New project dialog has Assembly/C; toolbar/menu say Build; target choice disabled for C; `highlight::Syntax` (C: keywords, types, `#` lines, comments over lines, strings, numbers, calls). Tests: smoke `c_project_builds_and_runs` (error line, save before build, breakpoint on a C line, monitor output), hbc2-cc `tests/build.rs` (`error_lines`, `c_lines`).

## Helpers (step 4)

The probe (every operator × every type, scratchpad probe/) gave exactly 89 helper names: chars are always widened to int except in assignment operators (`__pluseqc`…); long assignment operators get a long right operand (even shift counts). Helper conventions: ABI.md section 11. Runtime layout: comments at the top of each runtime file.

Old list:

From a program using every operator (scratchpad th.c): `__mul __div __divu __rem __remu __shl __shr __shru __cclt __ccltu __ccgt __ccgtu __cclteq __ccltequ __ccgteq __ccgtequ __pluseq __pluseqc __minuseq __muleq __divequ __remeq __shleq __shreq __andeq __oreq __xoreq __switch` and the long forms `__plusl __minusl __mull __divl __divul __reml __bandl __orl __xorl __negatel __cpll __shll __shrl __shrul __cceql __ccnel __ccltl __ccgtul __pluseql __minuseql __switchl` (+ the missing combinations: other compare/assign-op variants for c/uc/l/ul — generate the list systematically from backend.c make_node + helper_type). Also postinc/postdec variants when the result is used.

## Notes

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

## Session log

- 2026-10-31: step 10 (plan + choices told to the user first, decision 16). 10a-10e done, then C line stepping and background C builds (user's request). All workspace tests pass, clippy clean. Not committed.
- 2026-10-30: plan + decisions with the user. This file created.
- 2026-10-30 (same session): open questions settled (5, 6, 7, 10 above). `.bss` added to hbc2-asm (parser `Stmt::Bss`, assembler places it after code/data, `__bss_start`/`__bss_end`; tests `bss_*` in tests/assemble.rs; documented in the assembly manual). Step 1 done: kit copied to compiler/fcc, MSVC build works (all 5 tools), pipeline tested with the 6809 target (scratchpad) and with cc1.ref/cc2.ref (repo build). Label naming changed to `__L…` (told the user).
- 2026-10-31 (after step 9): `set_vector(port, handler)` added to hbc2.h (asm/hbc2.has; refuses port 0), test multi/vector (key handler in asm calling C; mutation-checked). ABI.md 8 no longer suggests fixed `.data` vectors (ports are dynamic, detection overwrites RTC/keyboard vectors); ABI.md 10 shows the install.
- 2026-10-31 (after step 9): register variables dropped (user). Step order changed: 10 IDE integration, 11 linker. Duplicate "Later: linker" removed.
- 2026-10-31 (step 9): multi-file builds (plan told to the user first; static prefix changed to `__S`, told). `cargo fmt --all` also reformatted 3 unrelated files (hbc2-ide build.rs/project.rs, hbc2-runner worker.rs, import order only); restoring them was blocked: left for the user.
- 2026-10-31 (step 8): library in 4 parts (hbc2.h + device detection in crt0, stdio putchar/puts/getchar/gets, string.h, printf), told the user after each. Decisions 12-15 recorded; step order now 9 multi-file, 10 linker, 11 IDE.
- 2026-10-31 (step 7): rules.hbc2, copt in both drivers, tests/peephole.c, coverage + mutation checks, psh/pop size count fixed.
- 2026-10-31 (later): step 6. Inline code + conditions on flags; gen_ops.py rewritten (class Program; groups ops/cond/const/assign × globals/locals → tests/ops/, because of the 64 KB limit); bench.c; programs.rs prints sizes/ticks and also reads tests/ops; step 3 cast/sign bug fixed. Step 7 explained to the user.
- 2026-10-31: steps 4 + 5. Runtime helpers written and tested; crate hbc2-cc (compile + tests); gen_ops.py → tests/ops_*.c; FCC file name bug fixed. Told the user: runtime ≈3.9 KB in every program until the linker (decision 1b).
- 2026-10-30 (same session): step 3. hbc2-asm: a name in `.data` = 2-byte address (`.data p table + 4`), test `data_addresses_mixed_with_bytes`, documented. target-hbc2.c, backend-hbc2.c, build.cmd builds cc1.hbc2/cc2.hbc2, runtime/crt0.has, tools/hbc2cc.py, tests/basics.c + frames.c (run OK: 0x04AC, 0x025C). ABI.md 11 updated (pop rules, switch table).
- 2026-10-30 (same session): step 2. ABI.md written; NAMELEN raised to 31 chars; checked: no struct by value in FCC, char args 1 byte / int in varargs, CPU interrupt and flag facts. Rule added: asm names starting with `_` share the C namespace (the user's interruptHandler.has uses `_iod_…`, `_loop` etc. → C can't use `iod_…`, `loop`; the assembler reports clashes).
