# EEPROM reference

1 MiB of storage that keeps its content when the power is off. Programs read
and write it through its ports.

| | |
|---|---|
| Class | `01 80` (mass storage, other) |
| Product ID | `0x0004` |
| Ports | 5 |
| Interrupts | none |
| Size | 1 MiB: addresses `$0x00000` to `$0xFFFFF` (20 bits) |

See the [peripherals overview](../README.md) for ports, interrupts and how
to find the EEPROM's first port (`eepromPort` in the project template).

The EEPROM keeps its content across power-on and reset: what a program
writes is still there after a restart. It doesn't start programs: the
HBC-2 starts from its boot ROM. On the hardware, the EEPROM is a file
(`eeprom.bin`) on the I/O processor's SD card.

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

## Reading and writing from a program

```
; a = the byte at EEPROM address $0x12345.
    lod d, eepromPort
    add d, 2                ; d = ADDR_0
    mov a, 0x01
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = ADDR_1
    mov a, 0x23
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = ADDR_2
    mov a, 0x45
    psh d, a
    cal mbx_out
    asp 2
    lod d, eepromPort       ; d = CMD
    mov a, 1                ; READ
    psh d, a
    cal mbx_out
    asp 2
    inc d                   ; d = DATA
    psh d
    cal mbx_in
    pop a
```

```
; Writes 0x55 at EEPROM address $0x12345 (address ports set as above).
    lod d, eepromPort
    inc d                   ; d = DATA
    mov a, 0x55
    psh d, a
    cal mbx_out
    asp 2
    dec d                   ; d = CMD
    mov a, 2                ; WRITE
    psh d, a
    cal mbx_out
    asp 2
```

## From C

`hbc2.h` has `eeprom_read`, `eeprom_write`, `eeprom_load` (EEPROM to RAM)
and `eeprom_save` (RAM to EEPROM), with 32-bit addresses
(`unsigned long`). They return -1 or 1 when there is no EEPROM or the
addresses go past `$0xFFFFF`. The whole EEPROM is free for the program's
data.

## In the IDE

The EEPROM is plugged in by default (Emulator menu, Peripherals) and starts
blank. Its memory can be viewed in the **Memory** view. The Emulator menu
saves its content to a 1 MiB file and loads it back. What a program writes
stays across **Reset**, new builds and loaded images, until the IDE closes.

From the command line, `hbc2 run main.has --eeprom eeprom.bin` starts with
that content.
