# Backend (step 3) — how it works

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- Rewrites (gen_rewrite_node): casts that change nothing removed; FUNCCALL(NAME) → T_CALLNAME; NAME/LOCAL/ARGUMENT/LABEL + constant merged; DEREF/EQ of LOCAL/ARGUMENT → T_LREF/T_LSTORE (argument offsets += argbase 2 + frame_len), of NAME → T_NREF/T_NSTORE, of LABEL → T_LBREF/T_LBSTORE (sizes 1, 2, 4).
- `sp` = bytes pushed since the frame; stack offsets = node value + sp. `work_size` = size of the last value, used by jfalse/jtrue (`or b, 0` / `mov i, a; or i, b` …; jtrue needs a jump over a jmp: no "jump if not zero" instruction).
- Labels: `__L<unit>_<n><tail>` (FCC's), `__X<unit>_<n>` (backend's own), `__T<unit>_<n>` literals and local statics, `__W<unit>_<n>` switch tables, `__W<unit>_<tag>_<entry>` case labels. Unit = cc2 argv[5] (default 1).
- Data: one pending object per area (A_DATA, A_LITERAL, A_BSS): FCC can start a string literal in the middle of another object (`char *s = "hi"`). Flushed when the code area comes back, when a new object starts in the same area, at the end. char values: printable runs as "…"; wider values as 0xNN bytes.
- `; export _name` comment lines: used by the static renaming (see "Multi-file builds").
- Helper calls: gen_helpcall pops the left operand (pop_left, rules in ABI.md 11), then `cal __`.
- Temporary driver: `compiler/tools/hbc2cc.py file.c -o file.has [--unit N] [--no-copt]` (cpp → cc0 → cc1.hbc2 → cc2.hbc2 → copt; cc2 opt level 0: our backend ignores it).
- Known: cc1 reports the float error at the wrong line (end of the declarations, e.g. line 3 for line 1).

