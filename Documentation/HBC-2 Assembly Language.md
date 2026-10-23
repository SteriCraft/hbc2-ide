# HBC-2 Assembly Language (`.has`)

This is the language of the HBC-2 assembler (`hbc2 asm`, and later the IDE).
For what each instruction does, see the *HBC-2 Instruction Manual*.

```
; Prints "Hi" in the top-left corner of the monitor (monitor on ports 1-5)
.define MONITOR_CMD 5
.define WRITE 1

.data message "Hi", 0              ; a string followed by a 0 byte

_start:
    cli                            ; no interrupt handler in this example
    mov d, 1                       ; monitor DATA_0 port: colours
    mov b, 0x0F                    ; white on black
    out d, b
    mov x, message.msb             ; [x:y] = address of message
    mov y, message.lsb
    mov c, 0                       ; c = column
next_char:
    lod a, [x:y]                   ; a = next character
    cmp a, 0
    jme done
    mov d, 2                       ; monitor DATA_1 port: character
    out d, a
    mov d, 3                       ; monitor POS_X port: column
    out d, c
    mov d, MONITOR_CMD
    mov b, WRITE
    out d, b                       ; write the character
    inc c
    inc y
    jmp next_char
done:
    jmp done
```

## Lines

Each line holds one of:

* an instruction: `mnemonic operand, operand`
* a label: `name:`, optionally followed by an address (`handler: $0xF000`) or by an instruction (`loop: inc a`)
* a directive: `.data`, `.define` or `.include`

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
| `'A'`, `'\n'` | Character: its ASCII code. |
| `"Hello"` | String (`.data` only): its ASCII codes, with no terminator. |
| `$0x9000` | Memory address. |

Escape sequences in characters and strings: `\n` `\r` `\t` `\0` `\\` `\'` `\"` `\xHH`.

## Operands

| Operand | Example | Meaning |
|---|---|---|
| register | `a` | A register. |
| register pair | `[x:y]` | Two registers forming a 16-bit address or value. The first one is the most significant byte. |
| value | `42`, `'A'`, `-1`, `name.msb`, `name.lsb` | An 8-bit value. `name.msb` / `name.lsb` are the high / low byte of the address of a label or variable. |
| address | `$0x9000`, `name`, `name + 2`, `name - 4` | A 16-bit memory address. A label or variable name stands for its address. |

The destination always comes first: `lod a, var` loads `a`, `str var, a` stores `a`,
`out d, a` writes `a` to the port whose number is in `d`.

### Forms of each instruction

The form decides the addressing mode.

| Instruction | Forms |
|---|---|
| `adc add and or sub sbb xor` | `op r, r` · `op r, value` · `op r, address` |
| `cmp` | `cmp r, r` · `cmp r, value` · `cmp r, [r:r]` |
| `mov` | `mov r, r` · `mov r, value` |
| `lod` | `lod r, address` · `lod r, [r:r]` |
| `str` | `str address, r` · `str [r:r], r` |
| `inc dec` | `op r` · `op [r:r]` · `op address` |
| `not` | `not r` · `not address` |
| `jmp jmc jme jmn jms jmz jmf cal` | `op address` · `op [r:r]` |
| `in out` | `in r, r` (r1 = port r2) · `out r, r` (port r1 = r2) |
| `int` | `int value` |
| `psh pop shl asr shr` | `op r` |
| `nop hlt irt ret clc cle cli cln cls clz clf stc ste sti stn sts stz stf psf pof` | no operand |

`op address` with `[r:r]` uses the pair as the address (for jumps: the address to jump to; for `inc`/`dec`: the memory byte to change).

## Labels and code blocks

```
_start:              ; required, always at $0x0300 (where the CPU starts)
    ...
loop:                ; continues the block above
    ...
handler: $0xF000     ; starts a block at $0xF000
    ...
```

* A label with an address starts a **block** of code at that address. `_start:` is always at `$0x0300` and must exist.
* A label without an address **continues the block above it**: code falls through from one to the next.
* If there is no block above (first label of a file, or right after an `.include`), the label starts a **floating block**, placed automatically.
* An instruction must have a label somewhere above it.

## `.data`: variables

```
.data counter 0                    ; 1 byte, placed automatically
.data table 1, 2, 3, 4             ; 4 bytes
.data message "Hello", '\n', 0     ; strings, characters and numbers can be mixed
.data vector $0x0100 handler.msb, handler.lsb   ; at a fixed address
```

A variable's name stands for its address: `lod a, counter`, `str table + 3, a`.

## Automatic placement

Floating blocks and `.data` without an address are placed in source order, in
the first free space large enough, starting at `$0x0300`: first all code
blocks, then all data. Fixed blocks and data are never moved; the assembler
reports any overlap.

## `.define`

```
.define MONITOR_ID 0x4A
.define VECTOR $0x0100
```

Every use of the name, in every file, is replaced by what follows it. Defines
can use other defines. A name can only be defined once.

## `.include`

```
.include "interruptHandler.has"
.include "lib/math.has"
```

Inserts another file at this point. The path is relative to the file that
contains the `.include`. A file is included once only: later `.include`s of
the same file are ignored, so libraries can include each other freely.

## EEPROM target

`hbc2 asm main.has --eeprom` produces a 1 MiB EEPROM image instead of a 64 KiB
RAM image. Addresses can then go up to `$0xFFFFF`. At power-on, the EEPROM
copies only `$0x0100-$0x04FF` and `$0xF000-$0xFFFF` into RAM, so:

* automatic placement only uses those ranges,
* the assembler warns about anything placed elsewhere (it stays in the EEPROM,
  for the program to read through the EEPROM's ports).

## Changes from the first IDE

| Before | Now |
|---|---|
| `:name` | `name:` |
| `:name $0xF000` | `name: $0xF000` |
| `[xy]` | `[x:y]` |
| operands separated by spaces or commas | operands and `.data` values separated by commas |
| including a file twice was an error | it is included once |
| | added: `name + n`, `'A'`, `0b…`, `-1`, mixed `.data` values, escapes, upper case |
