# HBC-2 Peripherals

Reference for the devices plugged into the HBC-2: how a program finds them,
talks to them and receives their interrupts. Each device has its own page:

| Device | ID | Ports | Interrupts | Page |
|---|---|---|---|---|
| Monitor | `0x4A` | 5 | none | [Monitor](Monitor/Reference.md) |
| Real-Time Clock | `0x95` | 7 | once per second, on its first port | [RTC](RTC/Reference.md) |
| Keyboard | `0xAA` | 2 | key pressed (first port), key released (second port) | [Keyboard](Keyboard/Reference.md) |
| EEPROM | `0xE1` | 5 | none | [EEPROM](EEPROM/Reference.md) |

Every device also sends two interrupts on port 0 when it is plugged in (see
[Detecting the devices](#detecting-the-devices)).

## The I/O Driver (IOD)

The CPU does not talk to the devices directly. Its `in` and `out` reach the
**I/O mailbox**, a small card in front of an **I/O processor** (an ESP32).
The I/O processor runs the IOD: **256 ports** numbered `0x00` to `0xFF`.

* **Port 0** is reserved for the IOD itself. No device ever gets it.
* Each device gets a block of **consecutive ports** when it is plugged in, the
  first free ones from port 1 up.
* Each port is an 8-bit **latch**: it holds the last value written to it, by
  the CPU or by the device.

The diagram `HBC-2_Input_Output.pdf` shows the mailbox and its ports.

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
.data16 port0_vector $0xFE00 iod_handler     ; port 0
```

## Detecting the devices

When a device is plugged in (at power-on in the emulator), the IOD sends
**two interrupts on port 0**:

1. the first one carries the device's **first port**,
2. the second one carries the device's **ID** (table above).

The device's other ports follow its first one. A port-0 handler therefore
remembers the port from the first interrupt and, on the second, stores it in
the variable that matches the ID. The project template
(`interruptHandler.has`, created with every new project) does exactly that:

| Variable | Holds |
|---|---|
| `monitorPort` | the monitor's first port (0 if none) |
| `rtcPort` | the RTC's first port (0 if none) |
| `keyboardPort` | the keyboard's first port (0 if none) |
| `eepromPort` | the EEPROM's first port (0 if none) |

Port 0 is never given to a device, so 0 means "not plugged (yet)". The
template also points the RTC's vector to a handler that does nothing, so the
RTC's ticks are harmless until the program installs its own.

The detection interrupts arrive at the very start, as soon as the program
runs with interrupts enabled. A program that needs a device waits for its
variable to change:

```
wait_monitor:
    lod a, monitorPort
    cmp a, 0
    jme wait_monitor
```

### Port numbers in the emulator

The emulator plugs the devices in this order: monitor, RTC, keyboard, EEPROM.
With all four, their ports are:

| Device | Ports |
|---|---|
| Monitor | 1-5 |
| RTC | 6-12 |
| Keyboard | 13-14 |
| EEPROM | 15-19 (only when the program is built for the EEPROM) |

The monitor, RTC and keyboard can be unplugged in the IDE's **Peripherals**
menu, which moves the next ones down: always use the detected ports rather than these numbers.
