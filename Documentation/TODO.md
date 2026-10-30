# Documentation to-do

## HBC-2 Monitor Instruction Manual

- **Displayable characters, "Colored text mode" and "ASCII Table" sections:** the
  manual says ASCII codes "32 to 127 (included)" are displayable. That is 96 codes,
  but the manual and `Peripherals/Monitor/Specifications.txt` both say 95 characters,
  and 127 is DEL (not displayable) in `Peripherals/Monitor/ASCII table.png`.
  Should read: **32 to 126** (0x20 space to 0x7E `~`).
- **Character list in the "ASCII Table" section** (also in `Specifications.txt`): the
  character after `_` is written `'` (apostrophe, 39), but code 96 is the backtick
  `` ` ``.

# Terminal to-do

## Command system (`lib/monitor/shell.has`, to build before the commands)

1. **Split the line into words** (`shell_parse`): go through `term_line`, replace
   each space with a 0 and save where each word starts (C-style word count + list
   of word addresses, 8 words at most). Lowercase the command word.
2. **Find and run the command** (`shell_exec`): a table of name + routine, ending
   with a 0:
   ```
   .data shell_cmds  "clear", 0, cmd_clear.msb, cmd_clear.lsb,
                     "memdump", 0, cmd_memdump.msb, cmd_memdump.lsb, ..., 0
   ```
   No indirect call in the ISA: push the routine's address, then `ret` into it.
   Unknown word: `Unknown command: xyz`.
3. **Argument helpers**: `shell_arg_num` reads word N as a number, decimal or hex
   (`0x...`; a hex text-to-number routine must be added to the math library).
   Standard errors: `Missing argument`, `Bad number`, `Out of range`. Print numbers
   with `utoa`/`htoa` + `term_print`.

Start with `help` and `clear` to test the 3 layers.

## Commands

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

Naming to decide: shorter names (`color 14`, `bgcolor 1`, or DOS-like `color 14 1`)
are quicker to type on a 42-column screen; the table also allows a second name.

## Later

- Up arrow: recall the last command (terminal feature).
- Load / save with the EEPROM.
