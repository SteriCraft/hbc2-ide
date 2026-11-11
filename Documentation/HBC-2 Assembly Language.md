# HBC-2 Assembly Language (`.has`)

This is the language of the HBC-2 assembler (`hbc2 asm`, and later the IDE).
For what each instruction does, see the *HBC-2 Instruction Manual*.

```
; Prints "Hi" in the top-left corner of the monitor (monitor on ports 1-5).
; The monitor is behind the I/O mailbox: put sends it the command
; "OUT: port d = b" (see "Devices and the I/O mailbox" below).
.define MONITOR_CMD 5
.define WRITE 1

.data message "Hi", 0              ; a string followed by a 0 byte

_start:
    cli                            ; no interrupt handler in this example
    mov d, 1                       ; monitor DATA_0 port: colours
    mov b, 0x0F                    ; white on black
    cal put
    mov x, message.msb             ; [x:y] = address of message
    mov y, message.lsb
    mov c, 0                       ; c = column
next_char:
    lod a, [x:y]                   ; a = next character
    cmp a, 0
    jme done
    mov d, 2                       ; monitor DATA_1 port: character
    mov b, a
    cal put
    mov d, 3                       ; monitor POS_X port: column
    mov b, c
    cal put
    mov d, MONITOR_CMD
    mov b, WRITE
    cal put                        ; write the character
    inc c
    inc y
    jmp next_char
done:
    jmp done

put:                               ; port d = b (changes a, i and the flags)
    mov a, 0x01                    ; the command: 0x01, the port, the value
    cal send
    mov a, d
    cal send
    mov a, b                       ; and on into send, which returns
send:                              ; sends a to the mailbox once it is free
    mov i, 1
    in i, i                        ; port 1: STATUS
    and i, 2                       ; bit 1: TX_FREE
    jmz send
    mov i, 0
    out i, a                       ; port 0: TX
    ret
```

## Lines

Each line holds one of:

* an instruction: `mnemonic operand, operand`
* a label: `name:`, optionally followed by an address (`handler: $0xF000`) or by an instruction (`loop: inc a`)
* a directive: `.data`, `.data16`, `.data32`, `.bss`, `.common`, `.define`, `.include`, `.keep` or `.library`

Comments start with `;` and run to the end of the line.

Mnemonics and registers are case-insensitive (`MOV A, B` = `mov a, b`).
Names (labels, variables, defines) are case-sensitive. A name starts with a
letter or `_`, followed by letters, digits or `_`. Mnemonics and register
names (`a b c d i j x y`) cannot be used as names.

## Values

| Written as | Meaning |
|---|---|
| `42`, `0x2A`, `0b0010_1010` | Number: decimal, hexadecimal or binary. `_` can separate digits. |
| `-1` | Negative number (8-bit values only: -128 to -1, stored as two's complement). |
| `'A'`, `'\n'`, `'é'` | Character: its code in the HBC-2's character set, Windows-1252 (ASCII, then `é` = `0xE9`, `œ` = `0x9C`, `€` = `0x80`…). |
| `"Hello"` | String (`.data` only): its Windows-1252 codes, with no terminator. |
| `$0x9000` | Memory address. |

Escape sequences in characters and strings: `\n` `\r` `\t` `\b` `\0` `\\` `\'` `\"` `\xHH`.

Source files are UTF-8. A character with no Windows-1252 code (`→`, `✓`…) is an
error in a literal (comments may hold anything); so is a control character typed
as is (a tab: write `\t`).

## Operands

| Operand | Example | Meaning |
|---|---|---|
| register | `a` | A register. |
| register pair | `[x:y]` | Two registers forming a 16-bit address or value. The first one is the most significant byte. |
| register pair + offset | `[x:y + 4]`, `[x:b + table.lsb]` | The address in the pair plus an offset from 0 to 255 (see [Offsets](#offsets)). |
| stack slot | `[sp]`, `[sp + 3]` | The byte at the stack pointer plus an offset from 0 to 255 (see [The stack](#the-stack)). |
| value | `42`, `'A'`, `-1`, `name.msb`, `name.lsb` | An 8-bit value. `name.msb` / `name.lsb` are the high / low byte of the address of a label or variable. |
| address | `$0x9000`, `name`, `name + 2`, `name - 4` | A 16-bit memory address. A label or variable name stands for its address. |

The destination always comes first: `lod a, var` loads `a`, `str var, a` stores `a`,
`out d, a` writes `a` to the port whose number is in `d`.

### Forms of each instruction

The form decides the addressing mode.

| Instruction | Forms |
|---|---|
| `adc add and or sub sbb xor` | `op r, r` · `op r, value` · `op r, address` · `op r, [r:r + n]` · `op r, [sp + n]` |
| `cmp` | `cmp r, r` · `cmp r, value` · `cmp r, [r:r + n]` · `cmp r, [sp + n]` |
| `mov` | `mov r, r` · `mov r, value` |
| `lod` | `lod r, address` · `lod r, [r:r + n]` · `lod r, [sp + n]` |
| `str` | `str address, r` · `str [r:r + n], r` · `str [sp + n], r` |
| `inc dec` | `op r` · `op [r:r + n]` · `op address` |
| `not` | `not r` · `not address` |
| `jmp jmc jme jmn jms jmz jmf cal` | `op address` · `op [r:r]` |
| `in out` | `in r, r` (r1 = port r2) · `out r, r` (port r1 = r2) |
| `int` | `int value` |
| `psh pop` | `op r` · `op r, r, …` (see [Register lists](#register-lists)) |
| `shl asr shr` | `op r` |
| `asp ssp` | `op value` (0-255) · `op value16` (256-65535) |
| `rsp` | `rsp [r:r]` |
| `wsp` | `wsp [r:r]` · `wsp address` |
| `nop hlt irt ret clc cle cli cln cls clz clf stc ste sti stn sts stz stf psf pof` | no operand |

`op address` with `[r:r]` uses the pair as the address (for jumps: the address to jump to; for `inc`/`dec`: the memory byte to change).

### Instruction sizes

An instruction takes 1 to 4 bytes: the CPU reads only the bytes its form
(its addressing mode) needs. The assembler places each instruction right
after the one before, so the size matters when counting bytes or when
computing an address from a label (`jmp loop + 3`).

| Size | Forms |
|---|---|
| 1 byte | no operand: `ret`, `nop`, `hlt`, `sti`… |
| 2 bytes | `op r, r` · `op r` · `in r, r` · `out r, r` · `jmp [r:r]` (and the other jumps, `cal`) · `rsp [r:r]` · `wsp [r:r]` |
| 3 bytes | `op r, value` · `op r, [sp + n]` · `str [sp + n], r` · `int value` · `asp value`, `ssp value` (0-255) |
| 4 bytes | `op r, address` · `str address, r` · `op address` (jumps, `cal`, `inc`, `dec`, `not`, `wsp`) · `op r, [r:r + n]` · `str [r:r + n], r` · `inc [r:r + n]`, `dec [r:r + n]` · `asp value16`, `ssp value16` |

### Offsets

`[x:y + n]` is the address in `[x:y]` plus `n`, from 0 to 255. The CPU adds
it on 16 bits: `[x:y + 0x20]` with `[x:y]` = `$0x12F0` is `$0x1310`, and
`$0xFFF0` + `0x20` wraps around to `$0x0010`. The registers are not changed.
`[x:y]` is the same as `[x:y + 0]`. The offset is a number, a character,
a define, or `name.msb` / `name.lsb`:

```
.define AGE 2                 ; field offsets of a record
    lod a, [x:y + AGE]        ; a = the record's age
    inc [x:y + AGE]
```

`name.lsb` as the offset indexes a table with an 8-bit register: with the
table's high byte in the first register, `[x:b + table.lsb]` is the
address of `table + b`, carry into the high byte included.

```
.data squares 0, 1, 4, 9, 16, 25, 36, 49

    mov x, squares.msb
    lod a, [x:b + squares.lsb]    ; a = squares[b]
```

Offsets are only added: `[x:y - 1]` is an error. Jumps, `rsp` and `wsp`
take a pair without offset.

### The stack

The stack pointer (SP, written STK in the Instruction Manual) starts at
`$0xFE00` and the stack grows downward: `psh` lowers SP by one, then writes,
so the first byte pushed is at `$0xFDFF`. SP always points at the last byte
pushed.

`[sp + n]` reads or writes the byte `n` bytes above the top of the stack
without moving SP: `[sp]` is the last byte pushed, `[sp + 1]` the one before.
`cal` pushes the return address (2 bytes), so a function finds the arguments
pushed by its caller above it:

```
    psh a                     ; first argument
    psh b                     ; second argument
    cal add2
    asp 2                     ; drop the two arguments

add2:                         ; a = first + second, using one local byte
    ssp 1                     ; reserve the local:  [sp] = local
                              ; [sp + 1], [sp + 2] = return address
                              ; [sp + 3] = second argument, [sp + 4] = first
    lod a, [sp + 4]
    str [sp], a
    lod a, [sp + 3]
    add a, [sp]
    asp 1                     ; free the local before ret
    ret
```

* `ssp n` moves SP down by `n` (reserves `n` bytes), `asp n` moves it up
  (frees them). Up to 255 the count fits in the instruction byte; up to 65535
  the assembler uses the 16-bit form.
* `rsp [x:y]` copies SP into `[x:y]`, for example to get the address of a local.
* `wsp [x:y]` or `wsp $0xC000` moves the stack somewhere else.

### Register lists

`psh` and `pop` accept several registers. This is a shorthand of the
assembler: each register becomes one `psh` or `pop` instruction.

```
copy16:
    psh a, i, j, x, y       ; psh a, psh i, psh j, psh x, psh y
    ...
    pop a, i, j, x, y       ; pop y, pop x, pop j, pop i, pop a
    ret
```

* `psh` pushes the registers in the order written.
* `pop` pops them in **reverse** order, so the same list restores what `psh` saved.
* A register can appear only once in a list. Register pairs are not allowed.

## Labels and code blocks

```
_start:              ; required, always at $0x0000 (where the CPU starts)
    ...
loop:                ; continues the block above
    ...
handler: $0xF000     ; starts a block at $0xF000
    ...
```

* A label with an address starts a **block** of code at that address. `_start:` is always at `$0x0000` and must exist.
* A label without an address **continues the block above it**: code falls through from one to the next.
* If there is no block above (first label of a file, or right after an `.include`), the label starts a **floating block**, placed automatically.
* An instruction must have a label somewhere above it.

## `.data`: variables

```
.data counter 0                    ; 1 byte, placed automatically
.data table 1, 2, 3, 4             ; 4 bytes
.data message "Hello", '\n', 0     ; strings, characters and numbers can be mixed
.data vector $0xFE02 handler.msb, handler.lsb   ; at a fixed address (port 1's interrupt vector)
```

A variable's name stands for its address: `lod a, counter`, `str table + 3, a`.

A name in `.data` (not `name.msb` / `name.lsb`) is its address on 2 bytes,
most significant first, as in `.data16`; `name + n` and `name - n` are allowed.
Addresses can be mixed with bytes and strings:

```
.data names first, second           ; 4 bytes: a table of two addresses
.data item 'A', 0x00, 0x05, names + 2   ; 1 + 2 + 2 bytes
```

### `.data16` and `.data32`: 16- and 32-bit values

```
.data32 population 100000          ; 0x00, 0x01, 0x86, 0xA0
.data16 offsets 1000, -1, 'A'      ; 0x03, 0xE8, 0xFF, 0xFF, 0x00, 0x41
.data16 vector $0xFE02 handler     ; handler.msb, handler.lsb
```

Each value takes 2 bytes (`.data16`) or 4 bytes (`.data32`), **most significant
byte first**. A value is one of:

* a number: 0 to 65535 (`.data16`) or 0 to 4294967295 (`.data32`). Negative
  numbers down to -32768 / -2147483648 are stored in two's complement.
* a character: `'A'`
* an address: `$0x9000`, or a label or variable name (`handler`, `table + 2`)

Strings and `name.msb` / `name.lsb` are only allowed in `.data`.

## `.bss`: zeroed variables

```
.bss buffer 64                     ; 64 bytes
.bss count 2
```

`.bss name size` reserves `size` bytes (1 to 65535; a number or a define) that
start at zero. Nothing is written in the program image for them: the program
clears them itself when it starts. The C compiler uses `.bss` for `static`
variables with no initial value (and `.common` for the others, see below).

All `.bss` variables are placed together, in source order, in one free area
after the code and data. They cannot have an address. The assembler defines
two names for the area:

* `__bss_start`: the address of its first byte,
* `__bss_end`: the address just after its last byte (equal to `__bss_start`
  when there is no `.bss`).

A third name, `__program_end`, is the address just after the last byte of
code or data placed automatically (fixed blocks and data are not counted),
for a loader that copies the program up to there. These names cannot be defined by the program. A startup
loop clears the area:

```
    mov x, __bss_start.msb         ; [x:y] = first byte
    mov y, __bss_start.lsb
    mov a, 0
clear:
    cmp x, __bss_end.msb
    jme clear_low
    jmp clear_byte
clear_low:
    cmp y, __bss_end.lsb
    jme clear_done
clear_byte:
    str [x:y], a
    inc y
    jmc clear_carry
    jmp clear
clear_carry:
    inc x
    jmp clear
clear_done:
```

### `.common`: a zeroed variable shared by several files

```
.common counter 2                  ; in a.has
.common counter 2                  ; in b.has: the same variable
```

`.common name size` is a `.bss` variable that may be written more than once,
in the same file or in other files. They all make one variable, as large as
the largest of them, placed with the `.bss` variables (after them, in the
order the names are first seen). A label, `.data` or `.bss` of the same name
is used instead of them; a `.data` or `.bss` smaller than the largest
`.common` is an error, and so is a label (code is not a variable). A
program's `.common` replaces a library's definition, like the program's
other definitions (`.library`).

The C compiler writes `.common` for a global variable that has no value
(`int n;`): C allows that line in several files, and they all refer to one
variable. `int n = 5;` in one of the files gives its value.

## Automatic placement

Floating blocks and `.data` without an address are placed in source order, in
the first free space large enough, starting at `$0x0000` and below the
interrupt vector table (`$0xFE00`): first all code blocks, then all data,
then the `.bss` area.
Fixed blocks and data are never moved; the assembler reports any overlap.

The stack grows down from `$0xFE00` and nothing stops it from reaching your
code or data: keep the top of memory free, or move the stack with `wsp`.

## Leaving out unused code

C programs are assembled with this on (the C library and the runtime are
large, and a program uses a small part of them). Assembly projects keep
everything unless you turn it on: an unused routine may be there on
purpose, to call it from the debugger. In the IDE: Build menu, "Leave out
unused code" (saved with the project). On the command line:
`hbc2 asm main.has --strip`. With it, a project can include a whole library
(`.include "lib/math/math.has"`) and carry only the routines it calls.

The program is cut into **pieces**: the code from one label up to the next
label, one `.data` variable, one `.bss` variable. A piece is kept when it is
used by a kept piece, starting from:

* `_start`,
* every block and `.data` at a fixed address (interrupt vectors, handlers);
  a block at a fixed address is kept whole,
* the names given to `.keep`.

A piece uses every name it refers to (`cal f`, `jmp loop`, `lod a, count`,
`mov x, table.msb`, `.data16 vector handler`…). A piece whose last
instruction is not `jmp`, `ret` or `irt` goes on into the next one (`hlt`
does too, and so does a label with no instruction), so it uses the piece
after it. Kept pieces of a block stay in the same order, one after the
other; the rest is gone, and so is its name. `__bss_start` and `__bss_end`
surround the kept `.bss` variables.

A name used but defined nowhere is an error even in a piece that is left
out: a typo in an unused routine still fails the build.

`hbc2 cc … --map` (or `hbc2 asm … --strip --map`) lists what was kept and
what was left out, with sizes. The IDE's build message gives the number of
bytes left out.

**Known limit.** Code that relies on where an unnamed spot is, past the next
label (`jmp table + 8` landing after the label that follows `table`), may
find something else there. Jump to a name instead.

### `.keep`

```
.keep handler
```

Keeps `handler` (and what it uses) even when no piece refers to it: for code
reached only through an address computed at run time. The name must be
defined somewhere.

### `.library`

```
.library
```

The names defined in the rest of the file, and in the files it includes, are
**library names**. When the program defines the same name, that is not an
error: the program's definition is used everywhere (in the library too), and
the library's is left out. A program can so write its own `putchar`, and the
library's `printf` calls it. Two library definitions of one name, or two
program definitions, are still an error. The C library starts with
`.library`; the runtime does not (its names start with `__`, which C
programs can't use, and `_start` can't be replaced).

A library piece that goes on into a replaced one (no `jmp`/`ret` between
them) still goes on into the library's code: only the name moves to the
program's definition.

## `.define`

```
.define SCREEN_COLUMNS 42
.define VECTOR $0xFE02
```

Every use of the name, in every file, is replaced by what follows it. Defines
can use other defines. A name can only be defined once.

## `.include`

```
.include "devices.has"
.include "lib/math.has"
```

Inserts another file at this point. The path is relative to the file that
contains the `.include`. A file is included once only: later `.include`s of
the same file are ignored, so libraries can include each other freely.
`..` goes up one folder: `.include "../lib/math.has"`. On Windows, letter case
does not matter: `Lib/Math.has` and `lib/math.has` are the same file.

## Devices and the I/O mailbox

`in` and `out` reach the **mailbox** card, not the devices. The devices
(monitor, RTC, keyboard, EEPROM) are behind an I/O processor, which gives
them their ports and raises their interrupts as before (see
`Peripherals/README.md`). The program asks it which devices there are (the
command INFO, below). The mailbox has two ports:

| Port | `in` | `out` |
|---|---|---|
| `0x00` | RX: the next byte from the I/O processor | TX: one byte for the I/O processor |
| `0x01` | STATUS: bit 0 RX_AVAIL, bit 1 TX_FREE, bit 2 EVT_PEND | nothing |

Ports `0x04` and `0x05`, `0x08` and `0x09`... up to `0x3C` and `0x3D` are
copies of them. Nothing answers on the other ports.

A device port is written with the command `0x01, port, value` and read with
`0x02, port`, which the I/O processor answers with one byte on RX. Each byte
goes to TX once STATUS shows TX_FREE, and the answer is read once it shows
RX_AVAIL. `compiler/lib/asm/mailbox.has` does it, with interrupts off during
the command, and keeps every register and flag:

```
    psh d, a                ; out d, a to a device:
    cal mbx_out             ;   port d = a
    asp 2

    psh d                   ; in a, d from a device:
    cal mbx_in              ;   a = port d
    pop a
```

A command takes effect a few microseconds later, but always before the next
command is read: an `in` after an `out` sees its result. The C library uses
these routines for `in()` / `out()` and all its drivers. A new assembly
project in the IDE gets its own copy of `mailbox.has`, already included by
`main.has`.

The command `0x03` (INFO) asks for the devices: the I/O processor replies
the protocol version, the number of devices, then 6 bytes per device (class,
product ID, first port, number of ports). `compiler/lib/asm/devices.has`
sends it at startup, waits 2 s at most, and finds devices by class:

```
    cal dev_init            ; the device table (empty without an I/O processor)
    mov a, DEV_INPUT        ; d = the first keyboard's first port,
    mov b, DEV_KEYBOARD     ;   0 when there is none
    cal dev_port
```

A new assembly project gets a copy of `devices.has` too ("Find the devices
at startup", on by default), and its `main.has` calls `dev_init`. See
`Peripherals/README.md`, "Detecting the devices".

## Boot ROM for the hardware CPU

The hardware CPU (the Logisim design in `logisim/`) copies a 64 KiB boot ROM
into RAM after every reset: the ROM holds the program's 64 KiB RAM image. It
is two AT28C256 EEPROMs of 32 KiB each, so the image is written as two files:

* `hbc2 asm main.has --boot-rom` (or `hbc2 cc … --boot-rom`) writes `main.bin`
  as usual, plus `main_lo.bin` (`$0x0000-$0x7FFF`, the low chip) and
  `main_hi.bin` (`$0x8000-$0xFFFF`, the high chip). With `-o out.bin` they
  are `out_lo.bin` and `out_hi.bin`.
* In the IDE: Assembler menu, "Export boot ROM…".

Each file is the chip's contents from its address 0, as an EEPROM
programmer (an Arduino one, for instance) writes it.

## Changes from the first IDE

| Before | Now |
|---|---|
| `:name` | `name:` |
| `:name $0xF000` | `name: $0xF000` |
| `[xy]` | `[x:y]` |
| operands separated by spaces or commas | operands and `.data` values separated by commas |
| including a file twice was an error | it is included once |
| | added: `name + n`, `'A'`, `0b…`, `-1`, mixed `.data` values, escapes, upper case, `psh`/`pop` register lists, `.data16`/`.data32`, `.bss`, `.common`, `.keep`, `.library`, leaving out unused code (C builds; asm projects when asked) |
| the program starts at `$0x0300` | it starts at `$0x0000` |
| every instruction took 4 bytes | 1 to 4 bytes (see [Instruction sizes](#instruction-sizes)) |
| interrupt vectors at `$0x0100` | at `$0xFE00` (port n: `$0xFE00 + 2n`); the stack starts at `$0xFE00` and grows down |
| | added: `[x:y + n]`, `[sp + n]` (n can be `name.lsb`), `asp`, `ssp`, `rsp`, `wsp` |
