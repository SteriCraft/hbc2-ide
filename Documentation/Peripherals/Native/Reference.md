# Native commands reference

**Status: spec approved 2026-11-11 (decisions at the end). Not implemented
yet.**

The I/O processor (the ESP32 behind the mailbox) is a small video card,
modelled on the BBC Micro's VDU commands and the Agon Light's VDP (FabGL).
Besides `OUT`, `IN` and `INFO`, it understands **native commands**: one
command byte, its arguments, and a reply only for the commands that read
something. Each command is close to one call of FabGL's `Canvas` API, so the
firmware stays a thin dispatcher.

Why: through the old monitor ports, one character costs 5 `OUT` commands
(15 bytes, about 0.5 ms) and clearing the screen about 3,000. A native
command is a few bytes, the drawing is done by the ESP32, and since most
commands have no reply the CPU can stream them back to back.

See [HBC-2_Input_Output.md](../../HBC-2_Input_Output.md) for the mailbox and
the byte protocol, and the [peripherals overview](../README.md) for ports,
interrupts and device detection.

## The screen

| | |
|---|---|
| Resolution | 256 × 192 pixels (a custom FabGL VGA mode), every coordinate on 1 byte |
| Colours | 16, the [monitor's palette](../Monitor/Reference.md#palette) (`0x0` black … `0xF` white) |
| Font | 8 × 8, so **32 × 24 text cells**; cell (x, y) is at pixel (8x, 8y) |
| Characters | `0x20`–`0xFF` Windows-1252 (ASCII, French accents); `0x00`–`0x1F` are control codes |
| Text colours at start | **bright yellow (`0xE`) on blue (`0x1`)** |
| Frame rate | 60 Hz (VSYNC event) |

There is **one picture**, a bitmap. Text and graphics draw on it: a
character is 8 × 8 pixels of the bitmap, drawn in the text colours, and
graphics can draw over text.

The I/O processor also keeps one **text cell** per character position (the
character and its colours), for `GET_CHAR` and `SCROLL`. Only the text
commands change the cells: a pixel drawn over a character doesn't.

### Characters

The character set is **Windows-1252**:

* `0x20`–`0x7E`: ASCII.
* `0xA0`–`0xFF`: Latin-1 (`é` = `0xE9`, `à` = `0xE0`, `ç` = `0xE7`,
  `«` = `0xAB`, `°` = `0xB0`…).
* `0x80`–`0x9F`: the Windows-1252 additions, among them the French ones
  Latin-1 lacks: `€` `0x80`, `…` `0x85`, `Œ` `0x8C`, `’` `0x92`, `“` `0x93`,
  `”` `0x94`, `–` `0x96`, `—` `0x97`, `œ` `0x9C`, `Ÿ` `0x9F`. The five codes
  Windows-1252 leaves undefined (`0x81`, `0x8D`, `0x8F`, `0x90`, `0x9D`) and
  `0x7F` are blank glyphs.
* Every code from `0x20` to `0xFF` can be redefined with `DEFCHAR`. The font
  is a RAM copy of the default one; `TEXT_RESET`, the reset button and
  power-on restore the default.

The default glyphs come from the public domain `font8x8` (Daniel Hepper:
`font8x8_basic` and `font8x8_ext_latin`), plus the Windows-1252 glyphs drawn
for the HBC-2.

**Source files are UTF-8.** The assembler and the C compiler convert string
and character literals to Windows-1252: `"Noël"` gives the bytes
`4E 6F EB 6C`, `'œ'` gives `0x9C`. A character Windows-1252 can't represent
is an error, with its line.

### Control codes

`PUTC` and `PUTS` handle these codes instead of drawing them:

| Code | Name | What it does |
|---|---|---|
| `0x08` | `\b` backspace | Cursor one cell back; from column 0 to column 31 of the row above; nothing at (0, 0). With a pending wrap: cancels it, the cursor stays on column 31. It doesn't erase (send `\b`, space, `\b`). |
| `0x09` | `\t` tab | Cursor to the next column that is a multiple of 8; from column 24 on (or with a pending wrap), to the start of the next row |
| `0x0A` | `\n` new line | Cursor to column 0 of the next row (as C's `putchar('\n')`); cancels a pending wrap, so a full row followed by `\n` moves down one row only |
| `0x0D` | `\r` return | Cursor to column 0 of the same row; cancels a pending wrap |
| other `0x00`–`0x1F` | | Nothing (reserved) |

### Cursor, wrap and scroll

* The cursor is a text cell, (0, 0) at the top left. `PUTC` and `PUTS` draw
  at the cursor and move it one cell right.
* **Deferred wrap** (as a VT100): a character drawn on column 31 leaves the
  cursor on column 31 with a **pending wrap**. The next printable character
  first moves the cursor to column 0 of the next row, then is drawn there.
  `\n`, `\r`, `\b` and `CURSOR` cancel the pending wrap (see the table above).
* **Scroll:** when the cursor must go below row 23 (a pending wrap followed
  by a character, `\n` or `\t` on the last row), the whole screen moves up
  one row (8 pixels, graphics included) and the new bottom row is cleared in
  the text background colour. The cursor stays on row 23.

So **writing the bottom-right cell does not scroll**: a program can fill the
whole screen with text. The screen scrolls only when the next character
comes.

## Commands

Bytes = command byte + arguments. CPU cost = (bytes sent + bytes replied)
× ~18 µs, the time for the CPU to move one byte through the mailbox (a
send loop: wait for `TX_FREE`, `out`); a reply byte costs about the same
(wait for `RX_AVAIL`, `in`). The time the ESP32 needs to draw is not
included: during a long drawing it simply reads the next byte later, and the
CPU waits on `TX_FREE` (nothing is lost).

All values are bytes. Colours use their low 4 bits (the high 4 are ignored).

### System (`0x00`–`0x0F`, unchanged)

| Opcode | Name | Arguments | Reply | Bytes | CPU cost |
|---|---|---|---|---|---|
| `0x00` | `NOP` | | | 1 | 18 µs |
| `0x01` | `OUT` | port, value | | 3 | 54 µs |
| `0x02` | `IN` | port | value | 2 + 1 | 54 µs |
| `0x03` | `INFO` | | version, N, N × 6 | 1 + 2 + 6N | |
| `0x04`–`0x0F` | reserved | | | | |

`INFO`'s protocol version becomes **2**: the I/O processor understands the
native commands. Version 1 (today's emulator) doesn't.

### Text (`0x10`–`0x2F`)

| Opcode | Name | Arguments | Reply | Bytes | CPU cost | FabGL |
|---|---|---|---|---|---|---|
| `0x10` | `PUTC` | c | | 2 | 36 µs | `drawGlyph` at the cursor, or the control code |
| `0x11` | `PUTS` | n, c₁ … cₙ | | 2 + n | 36 µs + 18 µs per char | `drawGlyph` per character |
| `0x12` | `CURSOR` | x, y | | 3 | 54 µs | (cursor kept by the firmware) |
| `0x13` | `COLOR` | fg, bg | | 3 | 54 µs | `setPenColor`, `setBrushColor` |
| `0x14` | `CLEAR` | bg | | 2 | 36 µs | `setBrushColor` + `clear` |
| `0x15` | `SCROLL` | n | | 2 | 36 µs | `setBrushColor(bg)` + `scroll(0, -8n)` |
| `0x16` | `DEFCHAR` | code, 8 rows | | 10 | 180 µs | (RAM font copy) |
| `0x17` | `GET_CHAR` | x, y | char, colours | 3 + 2 | 90 µs | (text cells) |
| `0x18` | `GET_CURSOR` | | x, y | 1 + 2 | 54 µs | (cursor) |
| `0x19` | `SHOW_CURSOR` | on | | 2 | 36 µs | blinking underline, by the firmware |
| `0x1A`–`0x1E` | reserved | | | | | |
| `0x1F` | `TEXT_RESET` | | | 1 | 18 µs | default font, colours, cursor; `clear` |

* **`PUTC c`** draws character c at the cursor in the text colours and
  moves the cursor, or runs control code c.
* **`PUTS n c₁ … cₙ`**: the same for n characters (0 to 255; 0 does
  nothing). A full row of 32 characters is 34 bytes, about 0.6 ms.
* **`CURSOR x y`** moves the cursor to column x (0–31), row y (0–23) and
  cancels a pending wrap. Off the grid, the command is ignored.
* **`COLOR fg bg`** sets the text colours for the next characters: fg for
  the character, bg for its background (the glyph's 0 bits are drawn, FabGL's
  `GlyphOptions().FillBackground(true)`). It also sets the background of the
  rows `SCROLL` clears.
* **`CLEAR bg`** fills the whole screen with colour bg, sets every text cell
  to a space with colours (fg, bg), and moves the cursor to (0, 0). It
  doesn't change `COLOR`.
* **`SCROLL n`** moves the screen up n rows (8n pixels, graphics included);
  the n rows at the bottom are cleared in the text background colour. The
  cursor and a pending wrap don't change. n = 0 does nothing; n ≥ 24 clears
  every row.
* **`DEFCHAR code r₀ … r₇`** changes the glyph of code (`0x20`–`0xFF`; a
  control code is ignored, its 8 bytes are still read). r₀ is the top row,
  bit 7 the leftmost pixel. Characters already on the screen don't change.
* **`GET_CHAR x y`** replies with the character of cell (x, y) and its
  colours, `bg << 4 | fg` (the old monitor's byte). Off the grid: `0x00`,
  `0x00`.
* **`GET_CURSOR`** replies with the cursor's column and row. With a pending
  wrap the column is **32** (past the end).
* **`SHOW_CURSOR on`**: 1 shows the cursor as an underline that blinks
  twice a second, drawn by the firmware without changing the cell; 0 hides
  it (default). The terminal no longer needs the RTC to blink its cursor.
* **`TEXT_RESET`** restores the default font and colours (14 on 1), hides
  the cursor, clears the screen in blue and moves the cursor to (0, 0).

### Graphics (`0x30`–`0x4F`)

| Opcode | Name | Arguments | Reply | Bytes | CPU cost | FabGL |
|---|---|---|---|---|---|---|
| `0x30` | `PIXEL` | x, y, c | | 4 | 72 µs | `setPixel(x, y, c)` |
| `0x31` | `LINE` | x₀, y₀, x₁, y₁, c | | 6 | 108 µs | `setPenColor` + `drawLine` |
| `0x32` | `RECT` | x, y, w, h, c | | 6 | 108 µs | `setPenColor` + `drawRectangle(x, y, x+w-1, y+h-1)` |
| `0x33` | `FILL_RECT` | x, y, w, h, c | | 6 | 108 µs | `setBrushColor` + `fillRectangle(x, y, x+w-1, y+h-1)` |
| `0x34` | `GET_PIXEL` | x, y | c | 3 + 1 | 72 µs | `waitCompletion` + `getPixel` (RGB → palette index) |
| `0x35`–`0x3F` | reserved | | | | | |
| `0x40` | `BLIT` | reserved | | | | `drawBitmap` |
| `0x41`–`0x4F` | reserved | | | | | |

* x is 0–255, all on screen; y is 0–191 on screen, 192–255 off screen.
  Whatever falls off the screen is clipped, nothing wraps around.
* **`LINE`** draws both end points.
* **`RECT`** / **`FILL_RECT`**: w or h = 0 draws nothing. The rectangle is
  w × h pixels from (x, y), clipped at the right and bottom edges.
* **`GET_PIXEL`** replies with the colour 0–15; off screen, 0.
* Graphics don't change the text cells, the cursor or the text colours.

### Sprites (`0x50`–`0x5F`): reserved

### Time (`0x60`–`0x6F`)

| Opcode | Name | Arguments | Reply | Bytes | CPU cost | ESP32 |
|---|---|---|---|---|---|---|
| `0x60` | `GET_TIME` | | year, month, day, hour, minute, second | 1 + 6 | 126 µs | its clock (NTP or a clock chip, still open) |
| `0x61` | `VSYNC` | on | | 2 | 36 µs | frame counter, counted on the VGA controller's vertical blank |
| `0x62`–`0x6F` | reserved | | | | | |

* **`GET_TIME`**: the values are the RTC's ports (year since 1900, month
  1–12, day 1–31, hour 0–23, minute, second). Without a clock, six zeros
  (month 0 is never a valid date). In the emulator, the RTC's time (the
  host's local time).
* **`VSYNC on`**: 1 sends an event every frame, 0 stops them (default).

### The VSYNC event

The I/O processor counts the frames since power-on, modulo 256. With
`VSYNC 1` it sends an event on the native display's port
([its descriptor](#device-descriptor)) at every frame, with the count as data
(in `I` in the handler).

* **At most one VSYNC event waits in the queue.** If the previous one hasn't
  been taken when the next frame comes (interrupts off for a while), the
  waiting event's data is updated to the new count instead of queuing a
  second one. The handler keeps the last count it saw and adds
  `(I - last) mod 256` to its tick counter, so no tick is lost and the queue
  never fills with frames.
* It becomes the system tick: `clock()` (`CLOCKS_PER_SEC` 60), delays.
* In the emulator a frame is every 2,000,000 / 60 CPU cycles (33,333 and
  33,334 in turn, 60 frames = 1 s at 2 MHz): emulated time, so a program
  behaves the same at any IDE speed, and tests are repeatable.

### Storage (`0x70`–`0x7F`): reserved

| Opcode | Name | Arguments | Reply |
|---|---|---|---|
| `0x70` | `SD_READ` | block number (4 bytes) | status, 512 bytes |
| `0x71` | `SD_WRITE` | block number (4 bytes), 512 bytes | status |

To be specified with the SD card.

### Other opcodes

`0x80`–`0xFF` and the reserved opcodes are ignored, as one-byte `NOP`s.
**A program must not send a native command to an I/O processor that doesn't
know it** (protocol version 1): its arguments would be read as commands. The
library checks the native display's descriptor first.

## Device descriptor

The native display needs a descriptor, for two reasons: programs must know
the I/O processor has native commands (with the version), and the VSYNC
event needs a **port** (its interrupt vector, `$0xFE00 + 2p`).

| | |
|---|---|
| Class | `03 80` (display, other) |
| Product ID | `0x0006` |
| Ports | 1: the VSYNC event's port; it holds the last frame count sent |

* A different subclass from the monitor (`03 00`), so `dev_find(DEV_DISPLAY,
  DEV_VGA)` still finds the old monitor ports and `dev_find(DEV_DISPLAY,
  0x80)` the native display. New names: `DEV_OTHER_DISPLAY` /
  `DEVICE_OTHER_DISPLAY`, `PRODUCT_DISPLAY`, and `display_port` (0 when
  there is none) set at startup, next to `monitor_port`.
* In the emulator it is plugged **last**, after the EEPROM (port 20 with all
  devices), so today's port numbers don't move. It comes and goes with the
  monitor (one screen): the IDE's Peripherals menu keeps one "Monitor" entry.
* Without it (no monitor plugged), the I/O processor still reads every
  native command whole and ignores it; `GET_CHAR` replies `0x00 0x00`,
  `GET_PIXEL` `0x00`, `GET_CURSOR` `0x00 0x00`, so the CPU never waits for
  a reply that doesn't come.

## The old monitor ports

The old 42-column text mode is gone. The monitor's 5 ports (`OUT` / `IN`
through `0x01` / `0x02`) stay, mapped onto the native picture:

| Old command | Text mode | Pixel mode |
|---|---|---|
| `WRITE` | Draws `DATA_1` in the colours `DATA_0` in cell (`POS_X`, `POS_Y`) and sets that text cell, as `CURSOR` + `COLOR` + `PUTC` would, but without moving the cursor or changing the text colours. `POS_X` > 31 or `POS_Y` > 23 is ignored. | `PIXEL` |
| `READ` | `GET_CHAR` | `GET_PIXEL` (`DATA_1` = 0) |
| `SWITCH_TO_PIXEL_MODE`, `SWITCH_TO_TEXT_MODE` | choose what `WRITE` and `READ` do; nothing changes on screen | |

* A control code written with `WRITE` is stored in the cell and drawn
  blank.
* Old programs that use columns 32–41 lose them; the 6 × 8 font is gone.
* The [Monitor reference](../Monitor/Reference.md) is rewritten in step 2.

The new library uses only the native commands.

## Reset and power-on

| | Power-on | Reset button (`rst_n`) | `TEXT_RESET` |
|---|---|---|---|
| Font | default | default | default |
| Text colours | 14 on 1 | 14 on 1 | 14 on 1 |
| Cursor | (0, 0), hidden, no pending wrap | (0, 0), hidden, no pending wrap | (0, 0), hidden, no pending wrap |
| Screen and text cells | blue, spaces | kept | blue, spaces |
| VSYNC events | off | off | unchanged |
| Frame count | 0 | kept | kept |
| Old monitor mode | text | text | unchanged |

VSYNC goes off at the reset button because the program restarts with its
vectors not yet set.

## Sending commands

* Each command is sent **with interrupts off** (as `OUT` / `IN`, see the
  [protocol](../../HBC-2_Input_Output.md#the-protocol)), so a handler's
  command never lands in the middle of another. A `PUTS` of 255 characters
  keeps interrupts off about 4.6 ms: keys wait in the queue, VSYNC events
  merge (above), nothing is lost. The library may split long strings.
* Commands without a reply are streamed: the CPU only waits for `TX_FREE`
  before each byte.
* The I/O processor runs the commands in order: a read (`GET_CHAR`,
  `GET_PIXEL`) sees every command sent before it.

### Examples (bytes sent)

```
11 05 48 65 6C 6C 6F        PUTS "Hello"            7 bytes, ~126 µs
13 0F 01                    COLOR white on blue     3 bytes
12 00 17                    CURSOR 0, 23            3 bytes
14 01                       CLEAR blue              2 bytes, ~36 µs (today ~0.3 s)
31 00 00 63 00 0C           LINE (0,0)-(99,0) red   6 bytes, ~108 µs
60                          GET_TIME -> 7E 0B 0B 0E 1E 00 (2026-11-11 14:30:00)
```

## In the emulator (step 2)

* The ESP32 model (`hbc2-core` `iop.rs`) runs the commands at once; each
  byte takes the I/O processor's usual service time (8 cycles). The time the
  ESP32 needs to draw is not modelled.
* The IDE's Monitor view shows the picture.

## Decisions (Gianni, 2026-11-11)

1. `0x80`–`0x9F` are Windows-1252. The assembler and the C compiler convert
   UTF-8 literals to Windows-1252, an error for any other character.
2. `GET_CURSOR`, `SHOW_CURSOR` and `TEXT_RESET` are kept.
3. Deferred wrap (VT100): writing the bottom-right cell doesn't scroll.
4. The old 42-column text mode is dropped; the old monitor ports map onto
   the native picture only.
5. `screen_set_mode` is removed from `hbc2.h`.
6. Default text colours: bright yellow (14) on blue (1), the palette's
   `#FFFF55` on `#0000AA`; power-on, the reset button and `TEXT_RESET`
   restore them.
7. The FabGL points to check on the board (vertical-blank hook, 256 × 192
   modeline at 60 Hz, `drawLine`'s pixels, `GET_PIXEL` read-back) are item
   B6 of `logisim/notes/breadboard-checks.md`.
