# Monitor reference

A screen with two modes: **text** (42 × 24 coloured characters) and **pixel**
(256 × 192 coloured pixels), both with a 16-colour palette.

| | |
|---|---|
| ID | `0x4A` |
| Ports | 5 |
| Interrupts | none (only the two plug-in interrupts on port 0) |
| Mode at power-on | text, every cell black with character code 0 (blank) |

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
| 1 | `WRITE` | Writes `DATA_0` (and `DATA_1` in text mode) at (`POS_X`, `POS_Y`) |
| 2 | `READ` | Puts what is at (`POS_X`, `POS_Y`) on `DATA_0` and `DATA_1` |
| 3 | `SWITCH_TO_PIXEL_MODE` | Shows the pixel screen |
| 4 | `SWITCH_TO_TEXT_MODE` | Shows the text screen |
| other | | Nothing |

A command runs each time the CPU writes `CMD`, with the values the other
ports hold at that moment. `CMD` keeps the value written: writing it again
runs the command again. The other ports keep their values too, so a program
only rewrites the ones that change (for example only `POS_X` and `DATA_1`
to write the next character of a line).

## Text mode

* 42 columns (`POS_X` 0-41) × 24 rows (`POS_Y` 0-23). Cell (0, 0) is the top
  left corner.
* Each character is 6 × 8 pixels, so the text screen is 252 × 192 pixels.
* Each cell holds two bytes:
  * colours: background in the high 4 bits, character colour in the low 4
    bits (`0x1F` = white on blue),
  * the character code.
* **Displayable characters:** codes `0x20` (space) to `0x7E` (`~`), 95 characters:

  ```
   !"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~
  ```

  Any other code is stored as written but shows as an empty cell in the
  background colour.

**WRITE** stores `DATA_0` (colours) and `DATA_1` (character) in the cell.
A position off the grid (`POS_X` > 41 or `POS_Y` > 23) is ignored.

**READ** puts the cell's colours on `DATA_0` and its character on `DATA_1`.
A position off the grid reads as `0x00` on both.

## Pixel mode

* 256 pixels wide (`POS_X` 0-255, every value is on screen) × 192 high
  (`POS_Y` 0-191). Pixel (0, 0) is the top left corner.
* Each pixel has a colour from 0 to 15.

**WRITE** sets the pixel to the low 4 bits of `DATA_0` (the high 4 bits are
ignored). `DATA_1` is not used. `POS_Y` > 191 is ignored.

**READ** puts the pixel's colour (0-15) on `DATA_0` and `0x00` on `DATA_1`.
`POS_Y` > 191 reads as `0x00`.

## Switching modes

The text screen and the pixel screen are two separate memories. Switching
mode only changes which one is shown: neither is cleared, and `WRITE` and
`READ` work on the memory of the current mode. Switching to the mode already
shown does nothing visible.

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

### Write a character

```
; Writes 'A' in white on blue at column 3, row 2.
    lod d, monitorPort      ; d = DATA_0
    mov a, 0x1F             ; blue background, white character
    out d, a
    inc d                   ; d = DATA_1
    mov a, 'A'
    out d, a
    inc d                   ; d = POS_X
    mov a, 3
    out d, a
    inc d                   ; d = POS_Y
    mov a, 2
    out d, a
    inc d                   ; d = CMD
    mov a, 1                ; WRITE
    out d, a
```

### Read a character back

```
; a = the character at column 3, row 2, b = its colours.
    lod d, monitorPort
    add d, 2                ; d = POS_X
    mov a, 3
    out d, a
    inc d                   ; d = POS_Y
    mov a, 2
    out d, a
    inc d                   ; d = CMD
    mov a, 2                ; READ
    out d, a
    lod d, monitorPort      ; d = DATA_0
    in b, d                 ; b = colours
    inc d                   ; d = DATA_1
    in a, d                 ; a = character
```

### Draw a pixel

```
; Switches to pixel mode and draws a bright red pixel at (100, 50).
    lod d, monitorPort
    add d, 4                ; d = CMD
    mov a, 3                ; SWITCH_TO_PIXEL_MODE
    out d, a
    lod d, monitorPort      ; d = DATA_0
    mov a, 0x0C             ; bright red
    out d, a
    add d, 2                ; d = POS_X
    mov a, 100
    out d, a
    inc d                   ; d = POS_Y
    mov a, 50
    out d, a
    inc d                   ; d = CMD
    mov a, 1                ; WRITE
    out d, a
```

## In the IDE

The screen is shown in the **Monitor** view. Pixel mode keeps one byte per pixel in the emulator; the real hardware would
pack two pixels per byte (24,576 bytes), which a program cannot see either
way.
