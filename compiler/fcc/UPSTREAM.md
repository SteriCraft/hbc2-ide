# Fuzix Compiler Kit (copy)

Core of the Fuzix Compiler Kit by Alan Cox and contributors:
<https://github.com/EtchedPixels/Fuzix-Compiler-Kit>, commit
`7b2a72edea6967e7df0b32e1e898fdea1754cc67` (2025-09-07).
Compiler licence: GPLv3 (see README.md, "Licence"). `copt.c` comes from Z88DK
(Clarified Artistic License). `Preprocessor/` comes from dev86 (see its COPYING).

Only the parts shared by every target were copied (plus `target-6800.c` and
`backend-default.c`, used as references); the other processor targets, the
runtime libraries and the tests were left out. HBC-2 files added here:
`target-hbc2.c` (parser target), `backend-hbc2.c` (code generator).

Changes to the upstream files (the Windows build itself needs none: it goes
through `../msvc/compat.h`, a forced include):

* `symtab.h`, `frontend.c`: names keep 31 characters instead of 15
  (`NAMELEN` 16 -> 32, `get_symbol_tail` reads `NAMELEN - 2` more bytes,
  `tokenize_symbol` buffer sized `NAMELEN`). Longer names are still cut.
* `frontend.c` (`directive`), `lex.c` (`T_LINE`): file names from `cpp`
  lines keep what follows the last `\` as well as the last `/` (Windows
  paths), and cc1 now reads the end marker of a 32-character name (it used
  to lose its place in the token stream: "useless declaration" errors).
* `expression.c` (`get_sizeof`): `sizeof` without a bracket takes a unary
  expression (`hier10`), not a whole expression: `sizeof a / sizeof a[0]`
  was read as `sizeof (a / sizeof a[0])` ("bad type").
* `tree.c` (`make_symbol` node): the node of a volatile variable (global
  or static: the ones `add_volatile` records) gets `SIDEEFFECT`, so the
  HBC-2 backend sees every use (reads, stores, `++`, `+=`) and lists the
  volatile names for the driver's reload removal.
* Source lines for the debugger (`;#line n` lines in the HBC-2 output):
  * `frontend.c` (cc0): the name of the C file goes out with the first `cpp`
    line (`write_line`, split out of `write_token`), even when a header
    comes before the file's first token.
  * `lex.c`, `lex.h`: `main_filename`, the first file name cc1 reads (the
    C file compiled).
  * `header.h`: `H_LINE` (data = the line, 0 = in another file than the C
    file). `body.c`, `body.h`, `main.c` (cc1): `line_header()` writes it
    before each statement, each local declaration, each top-level
    declaration, at the `}` of a function, and (line 0) before the
    variables declared without a value, which come at the end.
  * `backend.c`, `backend.h`: `H_LINE` calls the new backend hook
    `gen_line`; `backend-default.c` has an empty one.
  * `copt.c`: `;#line` lines are skipped when rules match, and the ones
    inside replaced lines are kept, after the new lines. The code is the
    same as without them.
