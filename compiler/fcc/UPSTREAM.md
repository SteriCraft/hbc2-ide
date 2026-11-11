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
* Struct and union assignment (HBC-2): `token.h` (`T_STRUCTCOPY`, left =
  the destination lval, right = the source address, value = the size),
  `expression.c` (`struct_assign`, called by `hier1` for `=`),
  `expression.h`, `initializer.c` (`struct s v = w;` for a local),
  `tree.c` (`make_rval` leaves a `T_STRUCTCOPY` as it is: an lval of the
  destination, so `a = b = c` and `(a = b).x` work). The backend
  (`backend-hbc2.c`) does the copy.
* Initial values of local arrays and structs (HBC-2): `declaration.c`
  (`one_declaration`: with `= {` or `= "…"` the storage comes after the
  values), `initializer.c` / `initializer.h` (`ini_auto_copy`: the values
  as hidden data under a new `label_tag`, then a `T_STRUCTCOPY` into the
  variable).
* `(void)` casts (HBC-2): `expression.c` (`hier10`): a `T_CAST` to `VOID`
  (a struct value gives its address, not read); the backend's cast of
  size 0 emits nothing.
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
* `float` and `double` (HBC-2, 2026-11-04): IEEE 754 binary32, `double` =
  `float` (`target-hbc2.c` remaps it):
  * `frontend.c` (cc0): decimal numbers (`dec_format`) and hexadecimal ones
    (`hex_format`) keep their text, and a float is converted by the host's
    `strtof` (correctly rounded, subnormals, hex floats `0x1.8p3`). Fuzix's
    `convert_fix32` (truncating, no subnormals, `p` read as a power of ten)
    and `parse_exponent` are gone. A leading 0 may start a decimal float
    (`0.5`, `0e0`, `017.5`); `017` stays octal, with an error for 8 or 9.
    The special case `-.5` (`tokenize_neg`, which flipped the sign twice:
    `-.5` was +0.5) is gone: `-` and `.5` are two tokens, and cc1 folds the
    negation. An `F` suffix on an integer converts it with the host's float.
  * `expression.c` (`step_constant`): the step of `++`/`--` on a float is
    the bits of 1.0f, not the integer 1.
  * `tree.c` (`constify`): casts of constants between integers and float
    (`fold_float_cast`), and `+ - * /` and the comparisons of two float
    constants (`fold_float_binary`) are folded with the host's float, so
    `float f = 1;`, `int i = 2.5;` and `1.0f / 0.0f` (`INFINITY`) are
    constants. A NaN result is 0x7FC00000.
* Block scope (HBC-2, 2026-11-11): `symbol.h` / `symbol.c` (a `depth`
  field set by `update_symbol`, the global `block_depth`;
  `update_symbol_by_name` reports "duplicate name" only for a local of the
  same depth and hides a local of an outer block), `body.c`
  (`statement_block`: depth + 1 inside `{ }`, and `mark_storage` /
  `pop_storage` around the block, so sibling blocks share stack room),
  `type_iterator.c` (`type_parse_function`: the arguments one deeper, the
  same depth as the body's outer `{ }`). Tags, typedefs and enum
  constants keep their file-wide scope.
