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
* a directive: `.data`, `.data16`, `.data32`, `.define` or `.include`

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

## Automatic placement

Floating blocks and `.data` without an address are placed in source order, in
the first free space large enough, starting at `$0x0000` and below the
interrupt vector table (`$0xFE00`): first all code blocks, then all data.
Fixed blocks and data are never moved; the assembler reports any overlap.

The stack grows down from `$0xFE00` and nothing stops it from reaching your
code or data: keep the top of memory free, or move the stack with `wsp`.

## `.define`

```
.define MONITOR_ID 0x4A
.define VECTOR $0xFE02
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
`..` goes up one folder: `.include "../lib/math.has"`. On Windows, letter case
does not matter: `Lib/Math.has` and `lib/math.has` are the same file.

## EEPROM target

`hbc2 asm main.has --eeprom` produces a 1 MiB EEPROM image instead of a 64 KiB
RAM image. Addresses can then go up to `$0xFFFFF`. At power-on, the EEPROM
copies only the first 512 bytes (`$0x0000-$0x01FF`) and the interrupt vector
table (`$0xFE00-$0xFFFF`) into RAM, so:

* automatic placement only uses `$0x0000-$0x01FF`,
* the assembler warns about anything placed elsewhere (it stays in the EEPROM,
  for the program to read through the EEPROM's ports).

Interrupt handlers are not copied: the code in the first 512 bytes has to
copy them (and anything else it needs) from the EEPROM into RAM before it
enables interrupts. See the EEPROM reference in `Peripherals/EEPROM/`.

## Changes from the first IDE

| Before | Now |
|---|---|
| `:name` | `name:` |
| `:name $0xF000` | `name: $0xF000` |
| `[xy]` | `[x:y]` |
| operands separated by spaces or commas | operands and `.data` values separated by commas |
| including a file twice was an error | it is included once |
| | added: `name + n`, `'A'`, `0b…`, `-1`, mixed `.data` values, escapes, upper case, `psh`/`pop` register lists, `.data16`/`.data32` |
| the program starts at `$0x0300` | it starts at `$0x0000` |
| interrupt vectors at `$0x0100` | at `$0xFE00` (port n: `$0xFE00 + 2n`); the stack starts at `$0xFE00` and grows down |
| | added: `[x:y + n]`, `[sp + n]` (n can be `name.lsb`), `asp`, `ssp`, `rsp`, `wsp` |
