# Monitor reference

The old monitor ports, kept so that older programs run. Since 2026-11-11
they reach the **native display**'s picture
([Native commands reference](../Native/Reference.md)): one picture of
256 × 192 pixels, 16 colours, with text in 8 × 8 cells, **32 × 24**. New
programs use the native commands, which are much faster (one character:
2 bytes instead of 5 port writes).

| | |
|---|---|
| Class | `03 00` (display, VGA) |
| Product ID | `0x0001` |
| Ports | 5 |
| Interrupts | none (the VSYNC event is the native display's) |
| At power-on | the picture is blue, every text cell a space in bright yellow on blue; the ports are in text mode |

See the [peripherals overview](../README.md) for ports, interrupts and how
to find the monitor's first port (`monitorPort` in the project template).

## Ports

| Offset | Name | Use |
|---|---|---|
| 0 | `DATA_0` | Colours: text mode `0xBF` (B = background, F = character colour); pixel mode `0x0C` (C = colour) |
| 1 | `DATA_1` | Text mode: character code. Not used in pixel mode. |
| 2 | `POS_X` | Column (text) or x (pixel) |
| 3 | `POS_Y` | Row (text) or y (pixel) |
| 4 | `CMD` | Writing it runs a command |

## Commands

| Value | Name | What it does |
|---|---|---|
| 0 | `NOP` | Nothing |
| 1 | `WRITE` | Text mode: draws `DATA_1` in the colours `DATA_0` in cell (`POS_X`, `POS_Y`). Pixel mode: sets pixel (`POS_X`, `POS_Y`) to `DATA_0`. |
| 2 | `READ` | Puts what is at (`POS_X`, `POS_Y`) on `DATA_0` and `DATA_1` |
| 3 | `SWITCH_TO_PIXEL_MODE` | `WRITE` and `READ` reach the pixels |
| 4 | `SWITCH_TO_TEXT_MODE` | `WRITE` and `READ` reach the text cells |
| other | | Nothing |

A command runs each time the CPU writes `CMD`, with the values the other
ports hold at that moment. `CMD` keeps the value written: writing it again
runs the command again. The other ports keep their values too, so a program
only rewrites the ones that change (for example only `POS_X` and `DATA_1`
to write the next character of a line).

There is **one picture**: switching mode changes nothing on screen, it only
chooses what `WRITE` and `READ` reach. Text drawn with `WRITE` is pixels of
the picture, and pixels drawn over a character don't change its cell.

## Text mode

* 32 columns (`POS_X` 0-31) × 24 rows (`POS_Y` 0-23). Cell (0, 0) is the top
  left corner, at pixel (0, 0); cell (x, y) is at pixel (8x, 8y).
* Each cell holds the colours (background in the high 4 bits, character
  colour in the low 4 bits: `0x1F` = white on blue) and the character code.
* **Characters:** Windows-1252, the native display's font: ASCII `0x20`–`0x7E`,
  Latin-1 `0xA0`–`0xFF` (`é` = `0xE9`), and `€ œ Œ Ÿ` and the typographic
  quotes in `0x80`–`0x9F`. A control code (below `0x20`) is stored as
  written and drawn blank.

**WRITE** draws the character in the cell and stores it. It doesn't move
the native display's cursor or change its text colours. A position off the
grid (`POS_X` > 31 or `POS_Y` > 23) is ignored.

**READ** puts the cell's colours on `DATA_0` and its character on `DATA_1`
(the native `GET_CHAR`). A position off the grid reads as `0x00` on both.

Columns 32-41 of the old 42-column screen are gone: programs written for it
lose what they put there.

## Pixel mode

* 256 pixels wide (`POS_X` 0-255, every value is on screen) × 192 high
  (`POS_Y` 0-191). Pixel (0, 0) is the top left corner.
* Each pixel has a colour from 0 to 15.

**WRITE** sets the pixel to the low 4 bits of `DATA_0` (the native `PIXEL`).
`DATA_1` is not used. `POS_Y` > 191 is ignored.

**READ** puts the pixel's colour (0-15) on `DATA_0` and `0x00` on `DATA_1`
(the native `GET_PIXEL`). `POS_Y` > 191 reads as `0x00`.

## Palette

| Value | Colour | RGB |
|---|---|---|
| `0x0` | Black | `#000000` |
| `0x1` | Blue | `#0000AA` |
| `0x2` | Green | `#00AA00` |
| `0x3` | Cyan | `#00AAAA` |
| `0x4` | Red | `#AA0000` |
| `0x5` | Magenta | `#AA00AA` |
| `0x6` | Brown | `#AA5500` |
| `0x7` | Light gray | `#AAAAAA` |
| `0x8` | Bright black (dark gray) | `#555555` |
| `0x9` | Bright blue | `#5555FF` |
| `0xA` | Bright green | `#55FF55` |
| `0xB` | Bright cyan | `#55FFFF` |
| `0xC` | Bright red | `#FF5555` |
| `0xD` | Bright magenta | `#FF55FF` |
| `0xE` | Bright yellow | `#FFFF55` |
| `0xF` | White | `#FFFFFF` |

## Examples

These examples use `monitorPort` from the project template.
They reach the ports with `mbx_out` and `mbx_in` (`compiler/lib/asm/mailbox.has`,
see [Reading and writing ports](../README.md#reading-and-writing-ports)).

### Write a character

```
; Writes 'A' in white on blue at column 3, row 2.
    lod d, monitorPort      ; d = DATA_0
    mov a, 0x1F             ; blue background, white character
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = DATA_1
    mov a, 'A'
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = POS_X
    mov a, 3
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = POS_Y
    mov a, 2
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = CMD
    mov a, 1                ; WRITE
    psh d, a
    cal mbx_out
    asp 2
```

### Read a character back

```
; a = the character at column 3, row 2, b = its colours.
    lod d, monitorPort
    add d, 2                ; d = POS_X
    mov a, 3
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = POS_Y
    mov a, 2
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = CMD
    mov a, 2                ; READ
    psh d, a
    cal mbx_out
    asp 2
    lod d, monitorPort      ; d = DATA_0
    psh d                   ; b = colours
    cal mbx_in
    pop b
    inc d                   ; d = DATA_1
    psh d                   ; a = character
    cal mbx_in
    pop a
```

### Draw a pixel

```
; Draws a bright red pixel at (100, 50).
    lod d, monitorPort
    add d, 4                ; d = CMD
    mov a, 3                ; SWITCH_TO_PIXEL_MODE
    psh d, a
    cal mbx_out
    asp 2
    lod d, monitorPort      ; d = DATA_0
    mov a, 0x0C             ; bright red
    psh d, a
    cal mbx_out
    asp 2
    add d, 2                ; d = POS_X
    mov a, 100
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = POS_Y
    mov a, 50
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = CMD
    mov a, 1                ; WRITE
    psh d, a
    cal mbx_out
    asp 2
```

## In the IDE

The picture is shown in the **Monitor** view, with the native display's
blinking cursor when a program shows it. The emulator keeps one byte per
pixel; the board keeps 4 bits per pixel (24,576 bytes), which a program
cannot see either way.
