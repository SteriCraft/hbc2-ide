# Multi-file builds (step 9) — how it works

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- `build_program(sources, &Library, dir) -> Build { program, compiled }`: `.c` compiled to `dir/<stem>_<unit>.has` (units 1, 2, … in list order), `.has` included as is; `dir/main.has` = crt0, runtime, library.has, sources; assembled (Target::Ram). `build_library` now returns `Library { path, files: (compiled, source) }`.
- Static renaming at the end of `compile()` (`rename_statics`; same code in hbc2cc.py): names defined (`_x:` label, `.data _x`, `.bss _x`, single `_`) without a `; export _x` line → `__S<unit>_x` on every identifier token, not in quotes/comments. Local statics are already `__T<unit>_n`.
- cpp gets `-I<source folder>` first (cpp finds the folder itself by the last `/`, which fails on the Rust driver's mixed `\`/`/` paths).
- Errors: hbc2-asm "already defined" now names the other file when it differs (`(file, line n)`); `c_message` swaps compiled paths for C files (drops the meaningless line), `'_x'`/`'__S2_x'` → `'x'`, "unknown label or variable" → "not defined:".
- Known: tentative `int n;` in two files is a duplicate (no common symbols); a `static` declared, never defined, but used would bind to another file's global (not checked).
- programs.rs: every folder in `tests/multi` is one program (sorted `.c`/`.has`, `main.c` holds "Expected", `main.screen`/`main.keys`).

