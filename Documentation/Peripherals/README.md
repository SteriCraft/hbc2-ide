# HBC-2 Peripherals

Reference for the devices plugged into the HBC-2: how a program finds them,
talks to them and receives their interrupts. Each device has its own page:

| Device | Class | Product ID | Ports | Interrupts | Page |
|---|---|---|---|---|---|
| Monitor | `03 00` | `0x0001` | 5 | none | [Monitor](Monitor/Reference.md) |
| Real-Time Clock | `08 03` | `0x0002` | 7 | once per second, on its first port | [RTC](RTC/Reference.md) |
| Keyboard | `09 00` | `0x0003` | 2 | key pressed (first port), key released (second port) | [Keyboard](Keyboard/Reference.md) |
| EEPROM | `01 80` | `0x0004` | 5 | none | [EEPROM](EEPROM/Reference.md) |

A program asks the I/O processor which devices there are (see
[Detecting the devices](#detecting-the-devices)) and finds them by class.

## The I/O Driver (IOD)

The CPU does not talk to the devices directly. Its `in` and `out` reach the
**I/O mailbox**, a small card in front of an **I/O processor** (an ESP32).
The I/O processor runs the IOD: **256 ports** numbered `0x00` to `0xFF`.

* **Port 0** is never given to a device, so a port variable at 0 means
  "no such device".
* Each device gets a block of **consecutive ports** when it is plugged in, the
  first free ones from port 1 up. Plugging a device sends no interrupt.
* Each port is an 8-bit **latch**: it holds the last value written to it, by
  the CPU or by the device.

The mailbox, the I/O processor and their protocol are described in
[HBC-2_Input_Output.md](../HBC-2_Input_Output.md), with the diagram
`HBC-2_Input_Output_diagram.pdf`.

### Reading and writing ports

A program asks the I/O processor to read or write a port with a command
through the mailbox (see "Devices and the I/O mailbox" in the *HBC-2 Assembly
Language* manual). The routines of `compiler/lib/asm/mailbox.has` do it and
keep every register and flag. The C library uses them; an assembly program
can include the file:

```
    psh d, a        ; port number in d, value in a:
    cal mbx_out     ;   the port now holds a
    asp 2

    psh d           ; a = the value the port number d holds
    cal mbx_in
    pop a
```

* `mbx_out` stores the value in the port. If the port is a device's command
  port (`CMD`), the device runs the command. It does so a few microseconds
  after `mbx_out` returns, but always before the I/O processor reads the next
  command: an `mbx_in` that follows sees the result. Writing any other port
  only stores the value, for the next command to use.
* `mbx_in` reads what the port holds. It never triggers anything.
* A command runs **each time** its `CMD` port is written, even with the same
  value as before.
* A plain `out d, a` or `in a, d` doesn't reach the devices: it goes to the
  mailbox's own ports (`0x00`, `0x01` and their copies).

Most programs keep each device's first port in a variable and add an offset
to reach the others, as the examples in these pages do:

```
    lod d, monitorPort      ; d = first port of the monitor (its DATA_0 port)
    add d, 4                ; d = its CMD port (offset 4)
```

### Interrupts

A device raises an interrupt **on one of its ports**, with one byte of data
(the value of that port at that moment). The I/O processor keeps the
interrupts in a **queue of 256**, oldest first; interrupts that arrive while
the queue is full are lost. It hands them to the mailbox one at a time.

The CPU takes the waiting interrupt when its Interrupt flag (I) is set,
between two instructions. For an interrupt on port `n`, it:

1. pushes register `I`, then the address of the next instruction (`PC`),
2. puts the interrupt's data byte in `I`,
3. clears the Interrupt flag (no other interrupt until `irt` or `sti`),
4. jumps to the address stored in the **interrupt vector table** for port `n`:
   most significant byte at `$0xFE00 + 2n`, least significant at
   `$0xFE00 + 2n + 1`.

`irt` returns: it pops `PC` and `I`, and sets the Interrupt flag again.

The CPU saves only `PC` and `I`. A handler saves and restores everything else
it changes, flags first (most instructions change them):

```
my_handler:
    psf                     ; save the flags
    psh a, b                ; save the registers used below
    ...                     ; i holds the interrupt's data
    pop a, b                ; restore in reverse order
    pof
    irt
```

**A vector that was never set holds `$0x0000`**, the start of the program:
an interrupt on that port restarts the program, without resetting anything.
Point the vector of every device that sends interrupts to a handler (even
one that is just `irt`) before allowing interrupts. The Interrupt flag is
**set at power-on**.

`hlt` stops the CPU until the next interrupt: a program waiting for a key or
the next second can sleep instead of looping. `hlt` doesn't turn interrupts
on: with interrupts off (`cli`), it stops the CPU until a reset. To wait with
interrupts off until then, write `sti` then `hlt`: `sti` takes effect after
the next instruction, so no interrupt can come between the two and be missed.

### Installing a handler

The vector of port `n` is at `$0xFE00 + 2n`, so for a port in `a`:

```
; Points the vector of the port in a to my_handler. Changes x, y.
install:
    psf
    cli                     ; no interrupt while the vector is half written
    mov x, 0xFE             ; [x:y] = $0xFE00 + 2 * a
    mov y, a
    shl y                   ; y = 2 * a (low byte), carry = the bit shifted out
    jmc install_high
    jmp install_write
install_high:
    inc x                   ; port 128 or more: $0xFFxx
install_write:
    mov a, my_handler.msb
    str [x:y], a
    mov a, my_handler.lsb
    str [x:y + 1], a
    pof                     ; interrupts back as they were
    ret
```

A vector at a port known in advance can also be written by the assembler:

```
.data16 port1_vector $0xFE02 my_handler      ; port 1
```

## Detecting the devices

At startup the program **asks** the I/O processor for its devices with the
command **INFO** (`0x03`). The reply comes on RX, one byte at a time:

1. the protocol version (1 byte, `1` today);
2. N, the number of devices (1 byte);
3. N descriptors of 6 bytes each, in port order:

| Bytes | Field |
|---|---|
| 2 | **class**: PCI base class, then subclass (see [Device classes](#device-classes)) |
| 2 | **product ID**, most significant byte first: which device it is (its ports and commands) |
| 1 | first port |
| 1 | number of ports |

The I/O processor (an ESP32) takes about 1 s to boot after power-on. A
command sent meanwhile just waits in TX, so the program doesn't need to know
whether it is ready: it sends INFO and waits for the reply. After a press
of the reset button (the CPU and the mailbox card only) the ESP32 is already
running and answers at once; it drops any command it was receiving when the
reset came.

`compiler/lib/asm/devices.has` does it all (a new assembly project gets a
copy, and the C library uses it):

* `dev_init` sends INFO and fills the **device table** (`dev_table`, 16
  devices at most; `dev_count`, `dev_protocol`). It waits **2 s at most**
  for each byte (a loop counted in CPU cycles: `DEV_TIMEOUT`). Without an
  answer, `dev_protocol` and `dev_count` stay 0: there is no I/O processor.
  It also points the vector of every port of every device to a handler that
  does nothing (`dev_ignore`), when the program left it at `$0x0000`.
* `dev_find` finds a device by class: `a` = base class, `b` = subclass
  (`DEV_ANY`: any), `c` = which one (0 = the first). C = 0 when found, with
  `d` = its first port and `[x:y]` = its descriptor; C = 1 and `d` = 0
  otherwise.
* `dev_port` is `dev_find` with `c` = 0.

The `main.has` of a new assembly project starts with:

```
_start:
    cli                         ; no event before the devices' vectors are set
    cal dev_init
    mov a, DEV_INPUT            ; the first keyboard
    mov b, DEV_KEYBOARD
    cal dev_port
    str keyboardPort, d         ; 0 when there is none
    ...
    sti
```

| Variable | Holds |
|---|---|
| `monitorPort` | the first display's first port (0 if none) |
| `rtcPort` | the first RTC's first port (0 if none) |
| `keyboardPort` | the first keyboard's first port (0 if none) |
| `eepromPort` | the first storage device's first port (0 if none) |

C programs get the same at startup (`compiler/runtime/crt0.has`): see
`hbc2.h` for `device_find()`, `device_get()`, `device_count()`,
`io_protocol()` and the variables `monitor_port`, `rtc_port`,
`keyboard_port`, `eeprom_port`.

Interrupts only come from the devices' events (a key, a clock tick...).

### Device classes

The class is the PCI class code (base class, subclass). The ones in use:

| Class | Meaning | `devices.has` | `hbc2.h` |
|---|---|---|---|
| `03 00` | display, VGA-compatible | `DEV_DISPLAY`, `DEV_VGA` | `DEVICE_DISPLAY`, `DEVICE_VGA` |
| `09 00` | input device, keyboard | `DEV_INPUT`, `DEV_KEYBOARD` | `DEVICE_INPUT`, `DEVICE_KEYBOARD` |
| `08 03` | system peripheral, real-time clock | `DEV_SYSTEM`, `DEV_RTC` | `DEVICE_SYSTEM`, `DEVICE_RTC` |
| `01 80` | mass storage, other | `DEV_STORAGE`, `DEV_OTHER` | `DEVICE_STORAGE`, `DEVICE_OTHER` |
| `02 xx` | network (none yet) | `DEV_NETWORK` | `DEVICE_NETWORK` |

The product ID tells two devices of one class apart (they may have other
ports and commands). Our own numbering; 0 is never used:

| Product ID | Device |
|---|---|
| `0x0001` | Monitor |
| `0x0002` | Real-Time Clock |
| `0x0003` | Keyboard |
| `0x0004` | EEPROM |
| `0x0005` | SD card (reserved, not in the emulator yet) |

### Port numbers in the emulator

The emulator plugs the devices in this order: monitor, RTC, keyboard, EEPROM.
With all four, their ports are:

| Device | Ports |
|---|---|
| Monitor | 1-5 |
| RTC | 6-12 |
| Keyboard | 13-14 |
| EEPROM | 15-19 |

Each device can be unplugged in the IDE's **Peripherals** menu, which moves the next ones down: always use the detected ports rather than these numbers.

In the emulator the I/O processor answers at once (no boot time).
