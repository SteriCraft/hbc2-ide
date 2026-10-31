# To-do

Everything planned or wished for, in one place. Done items are removed (git
history keeps them).

## Documentation

- **`docs/`** (the GitHub Pages site linked from README.md) is the Doxygen
  output of the old Qt IDE (2023). Replace it with a user guide for the Rust
  IDE (from `hbc2/README.md`), or remove it and the README link.

## Terminal (`lib/monitor/terminal.has`)

### Command system (`lib/monitor/shell.has`, to build before the commands)

1. **Split the line into words** (`shell_parse`): go through `term_line`,
   replace each space with a 0 and save where each word starts (C-style word
   count + list of word addresses, 8 words at most). Lowercase the command
   word.
2. **Find and run the command** (`shell_exec`): a table of name + routine,
   ending with a 0:
   ```
   .data shell_cmds  "clear", 0, cmd_clear.msb, cmd_clear.lsb,
                     "memdump", 0, cmd_memdump.msb, cmd_memdump.lsb, ..., 0
   ```
   No indirect call in the ISA: push the routine's address, then `ret` into
   it. Unknown word: `Unknown command: xyz`.
3. **Argument helpers**: `shell_arg_num` reads word N as a number, decimal or
   hex (`0x...`; a hex text-to-number routine must be added to the math
   library). Standard errors: `Missing argument`, `Bad number`, `Out of
   range`. Print numbers with `utoa`/`htoa` + `term_print`.

Start with `help` and `clear` to test the 3 layers.

### Commands

| Command | Notes |
|---|---|
| `help` | Lists the commands from the table |
| `clear` | `screen_clear` with the current colors, prompt on the top row |
| `set_text_color n` | Changes `term_color` for new text (0-15) |
| `set_bg_color n` | `screen_set_bg` on the whole screen, kept for new text |
| `memdump addr [len]` | 8 bytes per row: `1A00: 3E 00 12 ... >...` (hex + text); stop or pause when the output scrolls past the screen |
| `poke addr val` / `peek addr` | Write / read one byte of memory |
| `time` / `date` | Read the RTC ports |
| `settime hh mm ss` | RTC `SET_TIME` |
| `echo text` | Tests the word splitting |
| `hex n` / `dec n` | Number conversion: `hex 255` -> `0xFF` |
| `rand [min max]` | Uses `random_range` |
| `devices` | Port of each peripheral (monitor, keyboard, RTC, EEPROM) |
| `run addr` | Calls code at an address, back to the prompt after |
| `ver` | Version / about |

Naming to decide: shorter names (`color 14`, `bgcolor 1`, or DOS-like
`color 14 1`) are quicker to type on a 42-column screen; the table also
allows a second name.

### Later

- Up arrow: recall the last command.
- Load / save with the EEPROM.

## Math library (`lib/math`)

- Fixed point 8.8 and 16.16 (`lib/fixed_point`), with a sine table.
- A tests folder for the library.

## C compiler (`compiler/`, tracking file `compiler/ROADMAP.md`)

- **EEPROM:** C programs run from RAM only (ROADMAP decision 8); a boot
  path would let the HBC-2 start a program by itself.
- **float:** not supported; using it stops the build with an error.
- **Backend:** carry what the registers hold across loop labels (a dataflow
  version of `reloads.rs`; in bench.c's loops the paths into a loop label
  rarely agree: small gain expected).
- **Backend:** `x *= k` / `x /= k` in place, like `<<=`.
- **Build the C tools from cargo** (`cc` crate, one tiny bin crate per tool),
  so `cargo build` does everything without `compiler/build.cmd`.
- **Run FCC's own test programs** (upstream `test/`, not copied yet).
