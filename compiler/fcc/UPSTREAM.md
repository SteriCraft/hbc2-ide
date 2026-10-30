# Fuzix Compiler Kit (copy)

Core of the Fuzix Compiler Kit by Alan Cox and contributors:
<https://github.com/EtchedPixels/Fuzix-Compiler-Kit>, commit
`7b2a72edea6967e7df0b32e1e898fdea1754cc67` (2025-09-07).
Compiler licence: GPLv3 (see README.md, "Licence"). `copt.c` comes from Z88DK
(Clarified Artistic License). `Preprocessor/` comes from dev86 (see its COPYING).

Only the parts shared by every target were copied (plus `target-6800.c` and
`backend-default.c`, used as references); the other processor targets, the
runtime libraries and the tests were left out. HBC-2 files added here:
`target-hbc2.c`, `backend-hbc2.c` (not written yet).

Changes to the upstream files: none so far. The Windows build goes through
`../msvc/compat.h` (forced include) instead.
