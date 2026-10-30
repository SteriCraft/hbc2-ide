# HBC-2 Monitor Driver — Reference

Routines to draw on the HBC-2 monitor, in its two video modes: pixel mode
(256 × 192 pixels) and text mode (42 × 24 characters).

```
.include "interruptHandler.has"
.include "lib/monitor/driver.has"
```

`driver.has` uses the variable `monitorPort`, which `interruptHandler.has`
defines and sets, so that file must be included too. (`terminal.has`
already includes `driver.has`: a program that includes the terminal does
not include the driver again.)

Nothing runs at include time: the file only adds routines and a few
`.define`s.

---

## Contents

1. [Conventions](#1-conventions)
2. [Quick index](#2-quick-index)
3. [Before you start](#3-before-you-start)
4. [Colors](#4-colors)
5. [Video modes](#5-video-modes)
6. [Pixel mode](#6-pixel-mode)
7. [Text mode](#7-text-mode)
8. [Constants](#8-constants)
9. [Interrupts](#9-interrupts)
10. [Speed](#10-speed)
11. [How the driver talks to the monitor](#11-how-the-driver-talks-to-the-monitor)

---

## 1. Conventions

### Where the values are

| Value | Register |
|---|---|
| Position | `x` (column) and `y` (row) |
| Color (pixel mode) / colors (text mode) | `b` |
| Character | `a` |
| Text to write | address in `[i:j]`, length in `[c:d]` |

The same registers are used for input and output: `screen_get_pixel`
returns the color in `b`, the register `screen_put_pixel` takes it from.

### Registers

Every routine **restores every register except its outputs**. Only
`screen_write` moves its inputs forward (see
[section 7](#7-text-mode)).

### Flags

**C** is the only flag set by the routines. **C = 1 means nothing was
done**: no monitor, a position outside the screen, a color above 15 or a
character that cannot be displayed. Each routine lists its cases. Other
flags are undefined after the call: don't test them.

```
    cal screen_put_pixel
    jmc not_drawn           ; C = 1: the pixel was not drawn
```

---

## 2. Quick index

| Routine | Does | Mode | Section |
|---|---|---|---|
| `screen_pixel_mode` | switches the monitor to pixel mode | any | [5](#5-video-modes) |
| `screen_text_mode` | switches the monitor to text mode | any | [5](#5-video-modes) |
| `screen_put_pixel` | draws the pixel at `x`, `y` in color `b` | pixel | [6](#6-pixel-mode) |
| `screen_get_pixel` | `b` = color of the pixel at `x`, `y` | pixel | [6](#6-pixel-mode) |
| `screen_fill` | fills the whole screen with color `b` | pixel | [6](#6-pixel-mode) |
| `screen_putc` | draws character `a` at `x`, `y` in colors `b` | text | [7](#7-text-mode) |
| `screen_getc` | `a` = character, `b` = colors of the cell at `x`, `y` | text | [7](#7-text-mode) |
| `screen_write` | draws `[c:d]` characters read at `[i:j]`, from `x`, `y` | text | [7](#7-text-mode) |
| `screen_clear` | fills the screen with spaces in colors `b` | text | [7](#7-text-mode) |
| `screen_set_bg` | sets the background of every cell to color `a` | text | [7](#7-text-mode) |

---

## 3. Before you start

The monitor's ports are only known once it has been plugged in: the IOD
sends an interrupt and the handler in `interruptHandler.has` stores the
monitor's first port in `monitorPort`. Until then, `monitorPort` is 0 and
every routine returns with C = 1. (Port 0 is never given to a peripheral,
so 0 can mean "no monitor".)

A program that draws at start-up should enable interrupts and wait for the
monitor:

```
_start:
    sti                     ; let the IOD tell us about the peripherals
wait_monitor:
    lod a, monitorPort
    cmp a, 0
    jme wait_monitor        ; 0: the monitor is not known yet
```

---

## 4. Colors

The monitor has 16 colors, numbered 0 to 15:

| # | Color | # | Color |
|---|---|---|---|
| 0 | Black | 8 | Bright black (dark gray) |
| 1 | Blue | 9 | Bright blue |
| 2 | Green | 10 (0xA) | Bright green |
| 3 | Cyan | 11 (0xB) | Bright cyan |
| 4 | Red | 12 (0xC) | Bright red |
| 5 | Magenta | 13 (0xD) | Bright magenta |
| 6 | Brown | 14 (0xE) | Bright yellow |
| 7 | Light gray | 15 (0xF) | White |

* **Pixel mode**: a color is one of these numbers. The routines refuse
  values above 15 (C = 1).
* **Text mode**: a cell has two colors in one byte, **background in the
  high nibble, character in the low nibble**. `0x1F` = white (F) on blue
  (1), `0x07` = light gray on black. Every byte is valid.

---

## 5. Video modes

| Routine | Does | Flags |
|---|---|---|
| `screen_pixel_mode` | switches to pixel mode: 256 × 192 pixels | C = no monitor |
| `screen_text_mode` | switches to text mode: 42 × 24 characters | C = no monitor |

* **The monitor starts in text mode**, with every cell black.
* **Each mode keeps its own picture.** Switching mode shows the other
  picture as it was left; nothing is erased. A program can prepare the
  pixel picture, switch to text mode, then come back to it.
* **Use the routines of the current mode only.** The monitor cannot tell
  the program which mode it is in, so the routines cannot check. In the
  wrong mode they read or change the *other* picture: for example
  `screen_putc` in pixel mode changes a pixel near the top-left corner, and
  `screen_put_pixel` in text mode changes a character cell.

---

## 6. Pixel mode

256 × 192 pixels. **`x` = 0 to 255** (every value of `x` is on the screen),
**`y` = 0 to 191**. (0, 0) is the top-left corner.

| Routine | Does | Flags |
|---|---|---|
| `screen_put_pixel` | draws the pixel at `x`, `y` in color `b` | C = `b` > 15, `y` > 191, or no monitor |
| `screen_get_pixel` | `b` = color of the pixel at `x`, `y` (0 to 15) | C = `y` > 191, or no monitor |
| `screen_fill` | every pixel = color `b` | C = `b` > 15, or no monitor |

Details:

* On C = 1, **nothing changes**: no pixel is drawn, and `screen_get_pixel`
  leaves `b` as it was.
* `screen_fill` draws 49,152 pixels: it is slow (about 173,000 cycles, see
  [section 10](#10-speed)). To clear only part of the screen, draw that
  part with `screen_put_pixel`.

```
    cal screen_pixel_mode
    mov b, 1
    cal screen_fill         ; blue screen
    mov b, 15               ; white
    mov x, 0
    mov y, 0
diagonal:
    cal screen_put_pixel    ; pixel at (x, y)
    inc x
    inc y
    cmp y, SCREEN_HEIGHT
    jmf diagonal            ; y < 192: next pixel
```

---

## 7. Text mode

42 columns × 24 rows. **`x` = column, 0 to 41**; **`y` = row, 0 to 23**.
(0, 0) is the top-left cell. Each character is 6 × 8 pixels.

**Displayable characters**: ASCII 0x20 (space) to 0x7E (`~`), 95
characters. Control characters (below 0x20), DEL (0x7F) and codes above
0x7F are refused.

| Routine | Does | Flags |
|---|---|---|
| `screen_putc` | draws character `a` at `x`, `y` in colors `b` | C = `a` not displayable, (`x`, `y`) off-screen, or no monitor |
| `screen_getc` | `a` = character, `b` = colors of the cell at `x`, `y` | C = (`x`, `y`) off-screen, or no monitor |
| `screen_write` | draws `[c:d]` characters read at `[i:j]`, from `x`, `y`, in colors `b` | C = a character could not be drawn |
| `screen_clear` | every cell = a space in colors `b` | C = no monitor |
| `screen_set_bg` | background of every cell = color `a` (0 to 15) | C = `a` > 15, or no monitor |

Details:

* **`screen_putc`, `screen_getc`**: on C = 1 nothing changes (no cell is
  drawn, `a` and `b` stay as they were). `screen_putc` does not move `x`:
  to draw a line of characters, use `screen_write`.
* **`screen_clear`**: the high nibble of `b` is the background. The low
  nibble (character color) is stored in every cell too, but does not show
  on a space.
* **`screen_set_bg`** keeps every character and its character color; only
  the backgrounds change. It reads each cell before writing it back, so it
  is slower than `screen_clear`.

### `screen_write`

Draws a run of characters, left to right. The length is given in `[c:d]`
(0 to 65535): the text does not need a 0 at the end, and a 0 byte in the
text is not displayable (it stops the write, see below).

* **At the end of a row**, writing goes on at column 0 of the next row.
* **`[i:j]`, `[c:d]`, `x` and `y` move forward.** After the call, `[i:j]`
  points after the last character drawn, `[c:d]` = 0, and `x`, `y` are the
  cell after the last one drawn. The next `screen_write` (or
  `screen_putc`) can follow straight after. After the very last cell of
  the screen (41, 23), `x` = 0 and `y` = 24: one row below the screen.
  Save them first (`psh c, d, i, j, x, y`) if you need them again.
* **The screen does not scroll.** Writing past the bottom stops with C = 1.
  (`terminal.has` handles scrolling.)
* **C = 1** when a character cannot be drawn: it is not displayable, it
  would go below the screen, or there is no monitor. Writing stops there:
  `[i:j]` points at that character, `[c:d]` counts it with the ones left,
  and `x`, `y` are its cell. Everything before it has been drawn.
* `[c:d]` = 0 draws nothing and returns with C = 0.

```
.data hello "Hello, world!"

    cal screen_text_mode
    mov b, 0x1F
    cal screen_clear        ; white on blue, empty screen
    mov i, hello.msb
    mov j, hello.lsb        ; [i:j] -> the text
    mov c, 0
    mov d, 13               ; [c:d] = 13 characters
    mov x, 0
    mov y, 0                ; top-left cell
    cal screen_write        ; x = 13, y = 0: the next text follows on
```

---

## 8. Constants

`driver.has` defines these names; programs can use them too.

| Name | Value | Meaning |
|---|---|---|
| `SCREEN_HEIGHT` | 192 | pixel rows (pixel mode). The width, 256, needs no constant: every `x` fits. |
| `SCREEN_COLUMNS` | 42 | text columns |
| `SCREEN_ROWS` | 24 | text rows |
| `SCREEN_CMD_WRITE` | 1 | monitor command: write a pixel or cell |
| `SCREEN_CMD_READ` | 2 | monitor command: read a pixel or cell |
| `SCREEN_CMD_PIXEL_MODE` | 3 | monitor command: switch to pixel mode |
| `SCREEN_CMD_TEXT_MODE` | 4 | monitor command: switch to text mode |

---

## 9. Interrupts

The routines use only registers, the stack and the monitor's ports: no
variables. But each drawing is several `out` instructions in a row (color,
position, then command). If an interrupt handler also draws on the monitor,
it can change the ports in the middle of the main program's drawing, which
then lands in the wrong place or color.

Draw from one place only (the main program **or** the handlers), or disable
interrupts around the main program's calls (`cli` … `sti`).

---

## 10. Speed

Cycles for one call, `cal` and `ret` included, measured on the emulator.

| Routine | Cycles |
|---|---|
| `screen_pixel_mode` / `screen_text_mode` | 14 |
| `screen_put_pixel` | 23 |
| `screen_get_pixel` | 22 |
| `screen_fill` | 172,830 |
| `screen_putc` | 29 |
| `screen_getc` | 24 |
| `screen_write` | 9 + about 41 per character (1,733 for one full row) |
| `screen_clear` | 5,190 |
| `screen_set_bg` | 12,252 |

When a routine refuses its input (C = 1), it returns early: 6 to 12 cycles.

For a sense of scale: at 1 MHz, `screen_fill` takes about 0.17 s, and
`screen_clear` about 5 ms.

---

## 11. How the driver talks to the monitor

For writing your own routines. The monitor uses 5 consecutive ports,
starting at `monitorPort`:

| Port | Name | Pixel mode | Text mode |
|---|---|---|---|
| `monitorPort` + 0 | DATA_0 | color (0 to 15) | colors (background, character) |
| `monitorPort` + 1 | DATA_1 | — | character |
| `monitorPort` + 2 | POS_X | x | column |
| `monitorPort` + 3 | POS_Y | y | row |
| `monitorPort` + 4 | CMD | command (see [section 8](#8-constants)) | same |

* Writing CMD runs the command with the values already on the other ports.
  Those values stay until changed. To draw many pixels or cells in the
  same color, set DATA_0 (and DATA_1) once, then only POS_X / POS_Y and
  CMD: `screen_fill` and `screen_clear` work that way.
* READ puts the pixel's color, or the cell's colors and character, on
  DATA_0 / DATA_1, to read back with `in`.
* A position outside the screen is ignored by WRITE and reads as 0.
