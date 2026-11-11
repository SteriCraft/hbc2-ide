# HBC-2 Input / Output

How the HBC-2 talks to its devices: the I/O mailbox card, the I/O processor
(an ESP32) behind it, the byte protocol between them, and what happens at
start-up and reset. The diagram [HBC-2_Input_Output_diagram.pdf](HBC-2_Input_Output_diagram.pdf)
draws the card (source: `src/HBC-2_Input_Output_diagram.html`). Each device's ports
and commands are in [Peripherals/](Peripherals/README.md).

## Overview

```
 CPU  --in/out, INT/INR-->  mailbox card  --serial + flags-->  ESP32  -->  devices
      (data, address,       (12 TTL chips)                     (I/O        (VGA monitor,
       RW, IE, clk)                                             processor)  PS/2 keyboard,
                                                                            clock, EEPROM, SD)
```

* The **CPU** is unchanged: `in`, `out`, and the INT / INR exchange for
  interrupts.
* The **mailbox card** holds one byte each way and one pending event. It
  decodes two ports, `0x00` and `0x01`.
* The **ESP32** runs the devices. It keeps the 256 device ports (8-bit
  latches), gives each device its ports, and queues the devices' interrupts.
  The CPU reaches a device port with a command (`OUT`, `IN`) sent through the
  mailbox, not with a plain `in` / `out`.

There is no mouse and no audio. The planned board is a LilyGO TTGO VGA32; the
mailbox uses its SPI pins (shared with the SD card), its spare outputs and
four inputs (pins: `logisim/notes/io-redesign-proposal.md`, 9.4, to check
against the board's schematic).

## The mailbox ports

The CPU puts the port number on A0–A7 and sets IE (I/O Enable) with RW. The
card decodes only `0x00`–`0x3F`, and only A0, A1, A6 and A7:

| Port | `in` | `out` |
|---|---|---|
| `0x00` | RX: the next byte from the I/O processor | TX: one byte to the I/O processor |
| `0x01` | STATUS | nothing |
| `0x04`/`0x05`, `0x08`/`0x09` … `0x3C`/`0x3D` | copies of `0x00` / `0x01` (A2–A5 not decoded) | |
| `0x02`, `0x03` (and their copies) | a second chip select, not wired | |
| `0x40`–`0xFF` | nothing answers (`0xFF` in the emulator) | nothing |

STATUS:

| Bit | Name | 1 when |
|---|---|---|
| 0 | RX_AVAIL | a byte waits in RX |
| 1 | TX_FREE | TX is empty: the CPU can write the next byte |
| 2 | EVT_PEND | an event waits in EVT (INT is raised) |
| 3–7 | | always 0 |

## The mailbox card

| Part | Chips | Role |
|---|---|---|
| Decoding | 74HCT138 + 74HCT32 | selects TX, RX or STATUS from IE, RW, A0, A1, A6, A7; makes the TX write pulse |
| TX | 74HCT165 | loaded from the data bus by `out` on port 0; the ESP32 shifts it out |
| RX | 74AHCT595 | the ESP32 shifts a byte in; `in` on port 0 puts it on the data bus |
| STATUS | 74HCT244 | puts RX_AVAIL, TX_FREE, EVT_PEND on the data bus |
| EVT | 2 × 74AHCT595 | the event's port (on A0–A7) and data (on D0–D7) during the vector step |
| Flags | 4 × 74HCT74 | `tx_full`, `rx_full`, `evt_pend`, their clears, `VEC` (INR one step later), `evt_s` (= INT) |
| Levels | 74LVC244 | 5 V → 3.3 V towards the ESP32: MISO, `tx_full`, `rx_full`, `evt_pend`, `rst_n` |
| Pull-downs | 10 kΩ network | hold A8–A15 at 0 during the vector step |

The ESP32 drives SCK, MOSI, MBX_SEL, RCLK_RX and RCLK_EVT, and reads MISO and
the four flags.

### Sending a byte (CPU → ESP32)

1. The CPU reads STATUS until TX_FREE = 1.
2. `out` on port 0: the 74HCT165 loads the byte, `tx_full` goes to 1.
3. The ESP32 sees `tx_full`, pulls MBX_SEL low, reads 8 bits (SCK, MISO),
   then raises MBX_SEL: `tx_full` goes back to 0.

### Receiving a byte (ESP32 → CPU)

1. The ESP32 shifts 8 bits into the RX 595 (SCK, MOSI) and pulses RCLK_RX:
   `rx_full` = 1.
2. The CPU reads STATUS until RX_AVAIL = 1, then does `in` on port 0.
3. `rx_full` goes back to 0 in the next step.

### An interrupt (event)

1. The ESP32 shifts 16 bits into the EVT 595s (the data, then the port) and
   pulses RCLK_EVT: `evt_pend` = 1.
2. At the next clock edge `evt_s` copies `evt_pend`: that is INT.
3. If the I flag is set, the CPU ends its instruction and raises INR.
4. One step later (VEC) the EVT 595s put the port p on A0–A7 and the data d
   on D0–D7; A8–A15 are pulled to 0.
5. The CPU pushes I and PC, puts d in I, clears the I flag and jumps to the
   vector at `$0xFE00 + 2 × p`.
6. `evt_pend` goes back to 0 in the next step.

Handlers don't change from the old IOD: interrupt number = port, data in I.

**Firmware rule (I15):** the ESP32 waits at least 1 µs after RX's or EVT's
flag went back to 0 before writing that register again. So INT can't come
back on the very next cycle, and a byte is never overwritten before the CPU
read it.

## The ESP32 (I/O processor)

The firmware keeps the model of the old IOD:

* **256 device ports**, `0x00` to `0xFF`, each an 8-bit latch holding the last
  value written, by the CPU or by the device. These ports live in the ESP32,
  not on the bus.
* Each device gets a block of **consecutive ports**, the first free ones from
  port 1 up. **Port 0 is never given to a device**, so a port variable at 0
  means "no such device".
* A **queue of 256 events**, oldest first. An event that arrives while the
  queue is full is lost. Events go to the mailbox one at a time.
* The devices: VGA monitor, PS/2 keyboard, real-time clock (NTP once at
  start-up, or an I²C clock chip: still open), EEPROM (kept in a file), and later
  the SD card itself.

Plugging a device sends no interrupt; the CPU asks for the list (INFO).

## The protocol

The CPU sends commands of one or more bytes on TX. Replies come on RX, in
order. Events come on EVT and never mix with replies.

| Bytes sent | Command | Reply on RX |
|---|---|---|
| `0x00` | NOP | none |
| `0x01` p v | OUT: device port p = v | none |
| `0x02` p | IN: read device port p | 1 byte: the value of port p |
| `0x03` | INFO: the devices | protocol version, N, then N × 6 bytes |

* Any other command byte is ignored, as a NOP.
* Native commands (text, graphics, VSYNC, time), drafted for protocol
  version 2: [Peripherals/Native/Reference.md](Peripherals/Native/Reference.md).
* Writing a device's command port (`CMD`) with OUT runs the command each
  time, a few microseconds later, but always before the ESP32 reads the next
  command: an IN after an OUT sees its result.
* **Send each command with interrupts off** (`cli`, or `psf` / `cli` …
  `pof`). A handler that sent its own command in the middle of another
  would mix their bytes.

`compiler/lib/asm/mailbox.has` does it, keeping every register and flag
(about 210 cycles for an `mbx_out`):

```
    psh d, a        ; port d = a
    cal mbx_out
    asp 2

    psh d           ; a = the value of port d
    cal mbx_in
    pop a
```

### INFO

Reply, one byte at a time:

1. the protocol version (`1`);
2. N, the number of devices;
3. N descriptors of 6 bytes, in port order:

| Bytes | Field |
|---|---|
| 2 | class: PCI base class, then subclass |
| 2 | product ID, most significant byte first |
| 1 | first port |
| 1 | number of ports |

| Device | Class | Product ID | Ports |
|---|---|---|---|
| Monitor | `03 00` (display, VGA) | `0x0001` | 5 |
| Real-time clock | `08 03` (system, RTC) | `0x0002` | 7 |
| Keyboard | `09 00` (input, keyboard) | `0x0003` | 2 |
| EEPROM | `01 80` (storage, other) | `0x0004` | 5 |
| SD card (reserved) | `01 80` | `0x0005` | |

The class tells what a device does; the product ID tells which device it is
(its ports and commands). Programs find devices by class, never by fixed port
numbers. The full list of classes is in
[Peripherals/README.md](Peripherals/README.md#device-classes).

## Start-up and reset

**Power-on.** The boot ROMs copy the program into RAM and the CPU starts at
`0x0000`. The ESP32 takes about 1 s to boot, and it doesn't hold the CPU in
reset. The program doesn't need to know whether it is ready: it sends INFO
and waits for the reply, since a command sent meanwhile just waits in TX.

The program waits **2 s at most** for each byte, with a loop counted in CPU
cycles (`DEV_TIMEOUT_*` in `devices.has`: 4,000,000 cycles at 2 MHz). Without
an answer the device table stays empty and the protocol version reads 0: there
is no I/O processor.

**Reset button** (`rst_n`). It resets the CPU and the mailbox card's flags
only, and the ESP32 sees it on one of its inputs. The ESP32 keeps running and
keeps its device ports, its devices and its event queue. It drops the command
it was halfway through receiving and the replies the CPU hadn't read. The
program starts again, sends INFO, and gets the answer at once.

### In the software

* **Assembly:** `compiler/lib/asm/devices.has` (a new project gets a copy).
  `dev_init` sends INFO and fills `dev_table` (16 devices at most),
  `dev_count` and `dev_protocol`. It also points the vector of every device
  port the program left at `$0x0000` to a handler that does nothing.
  `dev_find` (class, subclass or `DEV_ANY`, which one) and `dev_port` find a
  device. The template's `main.has` sets `monitorPort`, `keyboardPort`,
  `rtcPort` and `eepromPort`.
* **C:** `crt0` does the same before `main` (`cli`, INFO, `sti`) and sets
  `monitor_port`, `keyboard_port`, `rtc_port`, `eeprom_port` (the EEPROM by
  product ID, because an SD card shares its class). `hbc2.h` has
  `struct device`, `device_find()`, `device_get()`, `device_count()` and
  `io_protocol()`.

## In the emulator

* The I/O processor answers at once: its boot time is 0 cycles by default
  (`IopConfig::boot_cycles`).
* Nothing answers on ports `0x40`–`0xFF`; `in` reads `0xFF`.
* The IDE's I/O view shows the mailbox (TX, RX, STATUS, EVT), the I/O
  processor (device ports, queue) and its log of the last 100 commands and
  events.
* The IDE's **Reset** (Ctrl+F5) is the reset button (`Machine::reset`);
  **Restart** (Ctrl+Shift+F5) is a power cycle (`Machine::power_on`).

## Not done yet

* The ESP32 firmware: the commands above, INFO's descriptors, dropping a
  half-received command on `rst_n`, the events, then native commands (text,
  drawing, SD card). On the Arduino Mega test rig first, then the TTGO VGA32.
