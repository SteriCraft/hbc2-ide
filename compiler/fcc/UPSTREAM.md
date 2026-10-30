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
