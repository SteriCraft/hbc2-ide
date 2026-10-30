# HBC-2 C compiler — roadmap (Claude's tracking file)

C compiler for the HBC-2 based on the Fuzix Compiler Kit (FCC):
`cc0` (tokens) → `cc1` (parse, trees) → `cc2` (our backend, prints `.has`) → `copt` (peephole) → hbc2-asm.
The user only says "go to the roadmap": read this file, continue at **Next**, update the status here at the end of each session.

## Status

- **Phase:** step 2 done: `compiler/ABI.md` written (examples of sections 5, 9, 10 run on the emulator).
- **Next:** step 3, `target-hbc2.c` + `backend-hbc2.c`, start from `backend-default.c` but with REAL push sizes (see notes), add cc1.hbc2/cc2.hbc2 to build.cmd. First decide the `.data` addition for `name + n` addresses (see design points).

## Decisions (agreed 2026-10-30)

1. **Linking:** (a) the driver joins every generated `.has` + the runtime into one program.
   Later (b): object files + linker in hbc2-asm, to drop unused code (memory usage).
2. **Names:**
   - C `foo` → `_foo` (so asm `screen_putc` and C wrapper `_screen_putc` coexist).
   - Runtime helpers: `__name` (Fuzix's names where they exist: `__mul`, `__cceq`…).
   - `static` names: per-file prefix `_S<n>_name` (n = file number in the build).
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
   - Every register except SP may be changed by a function (caller-saved). Register variables: maybe later.
5. **Asm library from C:** compiler helpers get their own entry points matching what the backend emits (built from the math library code); user-facing drivers (screen, terminal, keyboard) keep their register interface and get thin C wrappers.
6. **Types:** `char` unsigned by default (already FCC's default: `__castuc_`). `int`/`short` 16, `long` 32, pointers 16. No `float` for now: the compiler must STOP with a clear error at the C line (plan: `error()` in `target_sizeof` of `target-hbc2.c` for FLOAT/DOUBLE).
7. **Frames > 255 bytes:** compute the address with `rsp` (see notes), no compile error.
8. **EEPROM:** not handled now; the IDE loads the program into RAM. The user writes custom boot code later (C or asm) if needed.
9. **Interrupt handlers:** hand-written asm only (they may call C functions). No compiler support.
10. **Zeroed variables (BSS):** `.bss name size` in hbc2-asm (DONE 2026-10-30): all `.bss` placed together after code and data, nothing in the image, `__bss_start` / `__bss_end` defined; `crt0` clears the area (loop example in the assembly manual).
11. **Licence/packaging:** not a concern (single user). Repo is GPLv3 like the kit.

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
- **Statics:** cc2 prints `_s` for statics, just without `.export` (references don't say static/global). → backend prints a marker for exported names (e.g. `; export _g`) and the DRIVER renames non-exported `_name` symbols of that file to `_S<unit>_name`.
- **Arguments:** FCC's T_ARGUMENT offsets start at 0 for the first argument; `target_argsize` = real size (chars passed as 1 byte — fits `psh`).
- **Helpers:** default backend emits `call __enter`, `__loadl`, `__deref`, `__plus`, `__castuc_`, `__cleanup`… → names `__<op><type suffix>`. Our gen_helpcall prints `cal __`.

## Steps

- [x] 1. Get FCC, build it, compile tests for the 6809 target, read the backends. (No WSL/MSYS2/gcc on this machine: built with MSVC + compat layer instead. 6809 output checked on a loop + call test.)
- [x] 2. Write the ABI document — `compiler/ABI.md`. Section 11 (helpers) is a draft to settle in steps 3-4.
- [ ] 3. Minimal backend `backend-hbc2.c`: constants, load/store globals + locals, calls, prologue/epilogue, labels, jumps, branches; everything else → helpers.
- [ ] 4. Runtime: `crt0.has` (at 0x0000, SP already 0xFE00, `cal _main`, `hlt`) + helpers (mul/div/mod/shifts/compares/long ops) from the math library.
- [ ] 5. Test harness on hbc2-core (like the `rt` crates): compile → run → check result; run FCC's own test programs.
- [ ] 6. Inline code: 8-bit char ops, add/sub small constants, compare+branch with constants, `[x:y + n]` derefs, `+=`/`++` on locals.
- [ ] 7. `copt` peephole rules.
- [ ] 8. Library: `hbc2.h` (monitor/keyboard/RTC wrappers, in/out), `putchar`/`puts` on `term_putc`, `string.h`, small `printf`.
- [ ] 9. Multi-file builds (joined `.has`, static name prefixes).
- [ ] 10. IDE: `hbc2 cc` in hbc2-cli runs all stages; `.c` in projects; error lines; later asm→C line map for the debugger.

Later: linker (decision 1b), EEPROM, register variables, float.

## Notes

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

- 2026-10-30: plan + decisions with the user. This file created.
- 2026-10-30 (same session): open questions settled (5, 6, 7, 10 above). `.bss` added to hbc2-asm (parser `Stmt::Bss`, assembler places it after code/data, `__bss_start`/`__bss_end`; tests `bss_*` in tests/assemble.rs; documented in the assembly manual). Step 1 done: kit copied to compiler/fcc, MSVC build works (all 5 tools), pipeline tested with the 6809 target (scratchpad) and with cc1.ref/cc2.ref (repo build). Label naming changed to `__L…` (told the user).
- 2026-10-30 (same session): step 2. ABI.md written; NAMELEN raised to 31 chars; checked: no struct by value in FCC, char args 1 byte / int in varargs, CPU interrupt and flag facts. Rule added: asm names starting with `_` share the C namespace (the user's interruptHandler.has uses `_iod_…`, `_loop` etc. → C can't use `iod_…`, `loop`; the assembler reports clashes).
