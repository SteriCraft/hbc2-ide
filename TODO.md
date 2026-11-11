# To-do

Everything planned or wished for, in one place. Done items are removed (git
history keeps them).

## Documentation

- **`docs/`** (the GitHub Pages site) is the Doxygen output of the old Qt
  IDE (2023); README.md no longer links to it, but still takes its logo from
  `docs/documentation/html/logo.png`. Remove it (move the logo first) or
  replace it with a user guide for the Rust IDE.

## Terminal (`lib/monitor/terminal.has`)

**Argument helpers**: `shell_arg_num` reads word N as a number, decimal or
   hex (`0x...`; a hex text-to-number routine must be added to the math
   library). Standard errors: `Missing argument`, `Bad number`, `Out of
   range`. Print numbers with `utoa`/`htoa` + `term_print`.

Start with `help` and `clear` to test the 3 layers.

### Startup message

Must indicate the computer's name, shell version, memory capacity (RAM, EEPROM if present), list detected devices.

### Commands

| Status | Command | Notes |
|---|---|---|
|✔| `help [cmd]` | Lists the commands from the table |
|✔| `clear` | `screen_clear` with the current colors, prompt on the top row |
|✔| `set_colors <fg> <bg> [-f]` | Changes `term_color` for new text (0-15) |
|✔| `memdump <addr> [len]` | 8 bytes per row: `1A00: 3E 00 12 ... >...` (hex + text); stop or pause when the output scrolls past the screen |
|✔| `poke <addr> <val>` | Write one byte of memory |
|✔| `peek <addr>` | Read one byte of memory |
|✔| `date` | Read the RTC ports |
|✔| `time` | Read the RTC ports |
|✔| `echo text` | Tests the word splitting |
|✔| `hex n` / `dec n` | Number conversion: `hex 255` -> `0xFF` |
|✔| `rand [min max]` | Uses `random_range` |
|✔| `devices` | Port of each peripheral (monitor, keyboard, RTC, EEPROM) |
|✔| `run addr` | Calls code at an address, back to the prompt after |

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

- **float:** not supported; using it stops the build with an error.
- **Backend:** carry what the registers hold across loop labels (a dataflow
  version of `reloads.rs`; in bench.c's loops the paths into a loop label
  rarely agree: small gain expected).
- **Backend:** `x *= k` / `x /= k` in place, like `<<=`.
- **Build the C tools from cargo** (`cc` crate, one tiny bin crate per tool),
  so `cargo build` does everything without `compiler/build.cmd`.
- **Run FCC's own test programs** (upstream `test/`, not copied yet).
