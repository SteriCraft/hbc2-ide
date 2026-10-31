# EEPROM reference

1 MiB of memory that keeps its content when the power is off. At power-on it
fills the start of RAM, so the HBC-2 can start a program without a RAM image.

| | |
|---|---|
| ID | `0xE1` |
| Ports | 5 |
| Interrupts | none (only the two plug-in interrupts on port 0) |
| Size | 1 MiB: addresses `$0x00000` to `$0xFFFFF` (20 bits) |

See the [peripherals overview](../README.md) for ports, interrupts and how
to find the EEPROM's first port (`eepromPort` in the project template).

## Power-on

Before the CPU runs its first instruction, the EEPROM copies two ranges of
its memory into RAM, at the same addresses:

| EEPROM | RAM | Content |
|---|---|---|
| `$0x00000-$0x001FF` | `$0x0000-$0x01FF` | the first 512 bytes of the program (`_start` is at `$0x0000`) |
| `$0x0FE00-$0x0FFFF` | `$0xFE00-$0xFFFF` | the interrupt vector table |

The rest of RAM starts at zero. **Nothing else is copied, interrupt handlers
included**: the code in the first 512 bytes has to copy everything else it
needs from the EEPROM, through the ports (see [Boot loader](#boot-loader)).

The EEPROM keeps its content across power-on: what a program writes is still
there after a restart.

## Ports

| Offset | Name | Use |
|---|---|---|
| 0 | `CMD` | Writing it runs a command |
| 1 | `DATA` | The byte read, or the byte to write |
| 2 | `ADDR_0` | Address bits 19-16 (only the low 4 bits are used) |
| 3 | `ADDR_1` | Address bits 15-8 |
| 4 | `ADDR_2` | Address bits 7-0 |

## Commands

| Value | Name | What it does |
|---|---|---|
| 0 | `NOP` | Nothing |
| 1 | `READ` | `DATA` = the byte at the address in `ADDR_0`-`ADDR_2` |
| 2 | `WRITE` | The byte at the address in `ADDR_0`-`ADDR_2` = `DATA` |
| other | | Nothing |

A command runs each time the CPU writes `CMD`, with the address and data the
ports hold at that moment. All ports keep their values (`CMD` included): to
read the next byte, change `ADDR_2` (and `ADDR_1`, `ADDR_0` when they change)
and write `CMD` again. The address does not move by itself.

With `ADDR_0` = 0, EEPROM addresses `$0x00000-$0x0FFFF` match RAM addresses
`$0x0000-$0xFFFF`, which makes copying to the same RAM address easy.

## Building for the EEPROM

`hbc2 asm main.has --eeprom` (or the EEPROM target in the IDE) produces a
1 MiB EEPROM image. In the IDE, the EEPROM is plugged in when the program is
built for it, and a RAM image is not used then.

* Automatic placement only uses `$0x0000-$0x01FF`. Code and data meant to be
  copied later need a fixed address (`main: $0x0200`); labels without an
  address that follow it continue its block.
* The assembler warns about anything placed outside the two copied ranges.
  That is expected for code the boot loader copies: the warning is a
  reminder that it is not in RAM at power-on.
* `.data` at `$0xFE00-$0xFFFF` fills the interrupt vector table, which is
  copied at power-on.

## Boot loader

A program built for the EEPROM starts with a loader in the first 512 bytes.
It has to know the EEPROM's port before it can read anything, and that port
comes with the plug-in interrupts on port 0 (see
[Detecting the devices](../README.md#detecting-the-devices)). So the port-0
handler has to be in the first 512 bytes too, with its vector in the
interrupt vector table.

This example:

1. records the monitor's and the EEPROM's ports with a small port-0 handler,
2. waits for the EEPROM's port,
3. copies `main` (at `$0x0200` in the EEPROM) to `$0x0200` in RAM,
4. jumps to it.

```
; ===== Copied at power-on: the first 512 bytes and the vector table =====

.data16 port0_vector $0xFE00 boot_iod_handler   ; port 0's vector

.data boot_first 0          ; 1 between the 2 plug-in interrupts
.data boot_port 0           ; first port of the device being plugged
.data monitorPort 0
.data eepromPort 0

_start:
    lod a, eepromPort       ; interrupts are on at power-on: the handler
    cmp a, 0                ; below receives the plug-in interrupts
    jme _start
    cli
    mov x, main.msb         ; copy main ... main_end - 1
    mov y, main.lsb
    mov c, main_end.msb
    mov d, main_end.lsb
    cal boot_copy
    sti
    jmp main

; Copies the EEPROM's bytes [x:y] to [c:d] - 1 into RAM, at the same
; addresses (EEPROM $0x00000-$0x0FFFF). Changes a, i, j, x, y.
boot_copy:
    lod j, eepromPort       ; j = CMD
    mov i, j
    add i, 2                ; i = ADDR_0
    mov a, 0
    out i, a                ; addresses $0x0xxxx
boot_copy_loop:
    cmp x, c
    jme boot_copy_lsb
    jmp boot_copy_byte
boot_copy_lsb:
    cmp y, d
    jme boot_copy_done      ; [x:y] = [c:d]: done
boot_copy_byte:
    mov i, j
    add i, 3                ; i = ADDR_1
    out i, x
    inc i                   ; i = ADDR_2
    out i, y
    mov a, 1                ; READ
    out j, a
    mov i, j
    inc i                   ; i = DATA
    in a, i
    str [x:y], a
    add y, 1                ; next address
    adc x, 0
    jmp boot_copy_loop
boot_copy_done:
    ret

; Port 0: first interrupt = the device's first port, second = its ID.
boot_iod_handler:
    psf
    psh a
    lod a, boot_first
    cmp a, 0
    jme boot_iod_port
    mov a, 0                ; second interrupt: i = ID
    str boot_first, a
    lod a, boot_port
    cmp i, 0x4A             ; monitor
    jme boot_iod_monitor
    cmp i, 0xE1             ; EEPROM
    jme boot_iod_eeprom
    jmp boot_iod_end
boot_iod_monitor:
    str monitorPort, a
    jmp boot_iod_end
boot_iod_eeprom:
    str eepromPort, a
    jmp boot_iod_end
boot_iod_port:
    str boot_port, i        ; first interrupt: i = first port
    mov a, 1
    str boot_first, a
boot_iod_end:
    pop a
    pof
    irt

; ===== Copied by the boot loader =====

main: $0x0200
    ; ... the program: it can use monitorPort and eepromPort
    hlt
    jmp main
main_end:                   ; end of what boot_copy copies
```

`main_end` has no instruction: it only marks the address after the last byte
to copy. Everything between `main` and `main_end` (code, and `.data` with a
fixed address in that range) is copied.

Interrupts from the other devices go to their vectors as soon as they come.
A vector the image does not set is `$0x0000`, which restarts the program:
the RTC's first tick comes one second after power-on, and the keyboard's
when a key is pressed. Have `main` install their handlers first, or set
their vectors in the image if their ports are known.

## Reading and writing from a program

```
; a = the byte at EEPROM address $0x12345.
    lod d, eepromPort
    add d, 2                ; d = ADDR_0
    mov a, 0x01
    out d, a
    inc d                   ; d = ADDR_1
    mov a, 0x23
    out d, a
    inc d                   ; d = ADDR_2
    mov a, 0x45
    out d, a
    lod d, eepromPort       ; d = CMD
    mov a, 1                ; READ
    out d, a
    inc d                   ; d = DATA
    in a, d
```

```
; Writes 0x55 at EEPROM address $0x12345 (address ports set as above).
    lod d, eepromPort
    inc d                   ; d = DATA
    mov a, 0x55
    out d, a
    dec d                   ; d = CMD
    mov a, 2                ; WRITE
    out d, a
```

## From C

`hbc2.h` has `eeprom_read`, `eeprom_write`, `eeprom_load` (EEPROM to RAM)
and `eeprom_save` (RAM to EEPROM), with 32-bit addresses
(`unsigned long`). They return -1 or 1 when there is no EEPROM or the
addresses go past `$0xFFFFF`. A C program built for the EEPROM target
starts with a boot loader like the one above (`compiler/runtime/boot.has`):
it copies the whole program into RAM, so `main` and the interrupt
handlers set with `set_vector` work as with a RAM image. The program takes
the start of the EEPROM, at its RAM addresses: keep data from
`EEPROM_FREE` (`$0x10000`) on. See `compiler/ABI.md`, section 8.

## In the IDE

The EEPROM's memory can be viewed in the **Memory** view, and saved to a file
after the program changed it. Writes made by the program survive **Reset**;
they are lost when the IDE starts again from an image: a new build, a loaded
image, or a change in the Peripherals menu.
