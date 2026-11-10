# I/O redesign: replace the IOD with standard chips (proposal)

Status: **2026-11-10: the VIA / UART design (sections 2–5) failed its timing check (section 8). Gianni chose option C: a mailbox and an ESP32 I/O processor, proposed in section 9, waiting for review.** Nothing is changed in the code. Original status: **proposal for Gianni's review, 2026-11-10. Nothing is changed in the code.** The CPU does not change: IN / OUT, IE / RW, INT / INR and the vector table at `0xFE00` stay as they are (iod-requirements.md I1–I9).

Sources checked: `hbc2-core` (`cpu.rs`, `board.rs`, `iod.rs`, `machine.rs`, `peripherals/*.rs`), the microcode (`hbc2.uc`, interrupt entry IE1h–IE8), cu-decisions.md, addr-decisions.md, control-requirements.md C12 / C19 / C21 / C22, iod-requirements.md, the templates, the C library and runtime, the IDE and runner, and `Documentation/` (including the new `HBC-2_Input_Output.pdf` sketch).

## 0. Where the brief doesn't match the code (read first)

| # | Brief / sketch says | The code / CPU contract says | Proposed fix |
|---|---|---|---|
| F1 | The 74HCT244 is enabled by INR and drives `0xF0 + n` "during INR". | INR is high during the **last push step** (IE4h: `bus<-pchi, ram<-bus, lat<-0, inr`), when the CPU's address latch drives `bus_addr` to write PC high at SP. The **vector step** is the step **after** INR (IE5h, I6, C21). A 244 enabled by INR would fight the latch and corrupt the push. | Delay INR by one `clk` in a flip-flop: `VEC` = INR seen at the edge ending step n. The vector drivers are enabled by `VEC`, in step n + 1 only. |
| F2 | The 244 drives A0–A7. | In the hardware vector step the latch's outputs are **all off** (`abus` = 10, cu-decisions.md): the IOD must drive the whole `bus_addr`, `00:p` (I6, addr-decisions.md A8). | A second driver puts `00` on A8–A15 (74HCT541, inputs tied low). |
| F3 | Data on `bus_data` in the vector step: "0x00 or the IRQ states". | IE5h always does `i<-bus`: `bus_data` is loaded into register I, so it must be driven either way; `0x00` needs a driver too. | Third driver on `bus_data`. Same cost for `0x00` or the IRQ states, so the states are recommended (section 6, Q3). |
| F4 | INT = the 148's /GS, inverted. | I5: INT must come from a **flip-flop clocked by `clk`**, valid by 150 ns. The IRQ lines are asynchronous (the UART runs on its crystal), and the CPU has no synchronizer. | Sample the 8 IRQ lines with a 74HCT574 on `clk` (the "IRQ snapshot"). The 148, INT and the vector all come from the snapshot, so they are stable for the whole step. |
| F5 | INT stays high while the handler runs. | Fine for the hardware CPU: the I flag is cleared at IE1h and IE5h doesn't look at INT. But I6 says the IOD holds INT = 0 in the vector step, and the **emulator's handshake waits for INT low** (`CpuState::InterruptAcknowledge` in `cpu.rs`), so an INT that never drops would hang it. | INT = GS AND NOT `VEC` (one gate). Contract I6 and the emulator's CPU code both stay as they are. INT stays high in the handler, as you describe. |
| F6 | A 74HCT00 makes /RD and /WR for the chips. | The **W65C22 has no /RD and /WR**: it is a 6502-bus chip with PHI2, R/W, CS1, /CS2. PHI2 must also run all the time, because the timers count PHI2 cycles. | The VIA gets PHI2 = NOT `clk` and R/W = RW. /RD and /WR are for the 16C550 only. Timing is open (Q1). |
| F7 | /RD comes from RW, IE and a clock phase. | It has to. `bus_addr` and IE settle only about 72 ns into the step, and reading the 16C550 has side effects: RBR pops the FIFO, IIR clears THRE, LSR clears error bits. A glitch on /RD loses data. The window must also give data by 322 ns (I3). | /RD must start after the address and the 138 have settled. The window is open (Q2). |
| F8 | Monitor on ports 1–5. | True only with the default plug order. The IOD gives ports **at power-on, first free ones from port 1, in a fixed order** (monitor, RTC, keyboard, EEPROM: `Peripherals::power_on`). With all four plugged in: monitor 1–5, RTC 6–12, keyboard 13–14, EEPROM 15–19. Programs find them through the port-0 interrupts, not from fixed numbers. | Nothing to fix: just to know. |
| F9 | (not in the brief) | The **EEPROM peripheral copies `0000–01FF` and `FE00–FFFF` into RAM at power-on**. That models no real chip, and it does the same job as the hardware's **boot ROM** (2 × AT28C256, 64 KiB copied by the CPU: `boot_rom.rs`, `HARDWARE_BOOT_CYCLES`). | It goes away (section 3). |
| F10 | (not in the brief) | Today `in` reads a latch and **never triggers anything**, and a port reads back what `out` wrote. On real chips, reads have side effects and many registers are read-only or write-only. Programs that read back a port they wrote will break. | Part of the clean break (section 5). |
| F11 | 74HCT04: INTR, /GS, the 148's outputs. | That's 5 inverters. The 16C550's MR (reset) is **active high**, so it needs a 6th, and PHI2 = NOT `clk` needs a 7th. | One 74HCT04 is full. Use spare NAND gates or a second inverter chip; settle it in Logisim. |
| F12 | (not in the brief) | When no line is active, the 148 outputs `111` (active low), which gives **n = 0**. Noise or a source that drops its line during the 4 entry steps would land on vector `0xF0`. | Leave line 0 unconnected and point vector `0xF0` at a plain `irt` (Q5). |

Everything else in the brief matches: IN uses RW = 1, OUT uses RW = 0, and the port is on `bus_addr` as `00:port` (I3, I4). INR lasts one step (C21). The vector is `FE00 + 2p` (`IVT_START`, `enter_interrupt`). In the hardware vector step I := `bus_data`. IRT sets I with no delay, so a source that isn't cleared re-enters right after IRT. Software `INT n` goes through its own vector step (IE5s, the pending port latch) and doesn't touch the I/O board.

## 1. Current state (from the code)

### The IOD model (`hbc2-core/src/iod.rs`, `board.rs`)

- **256 port latches** (`ports: [u8; 256]`) plus an owner table (`owners`, device ID per port, 0 = free).
- **OUT** (`cpu.rs` `O::Out`): `iod.cpu_write(port, value)` sets the latch and records a pending write. After the CPU tick, `Machine::tick` passes it to the owning peripheral (`Peripherals::port_written`), which reacts to writes on its CMD port.
- **IN** (`O::In`): `iod.port(port)`, a plain read of the latch with no side effect.
- **Port 0** (`RESERVED_PORT`) is never given to a device. `Iod::connect` takes `count` consecutive ports from the first free one and sends **two interrupts on port 0**: data = the first port, then data = the device ID.
- **Interrupt queue**: a `VecDeque` of `(port, data)`, at most 256 (`QUEUE_CAPACITY`). `trigger_interrupt(port)` queues the port's **current latch value** as the data. When the queue is full, the interrupt is dropped (returns false).
- **Handshake** (`Board::tick_iod`, `Cpu::tick_with`): while INR = 0, INT = the queue isn't empty. At a boundary with I = 1 and no STI delay, the CPU raises INR (`InterruptAcknowledge`). On the next IOD tick, the IOD lowers INT and pops one entry: `address_bus = port`, `data_bus = data`. The CPU sees INT low, drops INR and calls `enter_interrupt`: push I, push PC (LSB then MSB), **I := data**, I and H flags cleared, PC := `mem16[0xFE00 + 2 × port]`. 9 cycles in total (`HARDWARE_INTERRUPT_CYCLES`).
- **Software `INT n`**: kept pending until I = 1, after any hardware interrupt. The data is register I itself, so I is unchanged. 10 cycles.

### Peripherals (`hbc2-core/src/peripherals/`)

| Device | ID | Ports (offsets) | Default ports | Interrupts (number = port, data → I) |
|---|---|---|---|---|
| (IOD) | — | port 0 | 0 | port 0 ×2 at plug-in: first port, then ID |
| Monitor | `0x4A` | 5: DATA_0, DATA_1, POS_X, POS_Y, CMD (NOP, WRITE, READ, PIXEL, TEXT) | 1–5 | none |
| RTC | `0x95` | 7: YEAR, MONTH, DAY, HOUR, MINUTE, SECOND, CMD (GET, SET) | 6–12 | first port, every second (data = YEAR) |
| Keyboard | `0xAA` | 2: PRESSED, RELEASED | 13–14 | first port on a press, second on a release (data = scan code) |
| EEPROM | `0xE1` | 5: CMD, DATA, ADDR_0, ADDR_1, ADDR_2 (1 MiB) | 15–19 (only when the build target is EEPROM) | none |

`MachineConfig` defaults to monitor + RTC + keyboard, and adds the EEPROM when the image is 1 MiB. The RTC follows host time (`advance_time`, `sync_rtc`). The runner sends its interrupts only at speeds where that makes sense (`sends_rtc_interrupts`). Keys go from the IDE through `keymap.rs` (AZERTY scan codes) to `Machine::key_event`.

### Other users of the IOD model

- **CPU verification**: `logisim/tools/emutrace` and `hbc2-ucode/src/check.rs` inject the bench stub's interrupts through `iod.set_port` + `trigger_interrupt` (the Logisim bench stub U16: port latches in a RAM, OUT to `F0+p` arms an interrupt). `hbc2-ucode/src/model.rs` has its own copy of the stub. These must keep working, because the CPU and its bench don't change.
- **Tests**: `hbc2-core/tests/interrupts.rs` (20 functions), `peripherals.rs` (19), `cpu.rs`, `cycles.rs`, `hbc2-runner/tests/runner.rs`, and `hbc2-cc/tests/programs.rs` (C tests with `.screen` / `.keys` files, run with monitor + RTC + keyboard).

## 2. Proposed hardware (target for the `iod` module)

```
bus_addr A4–A6 ──► 74HCT138 (G1 = IE, /G2A = A7, /G2B = GND) ──► /CS0 … /CS7
                        /CS0 → VIA CS2B (CS1 tied high)    /CS1 → UART /CS2 (CS0, CS1 tied high)
RW, IE, clk / clk_wr_n ─► gates ─► UART /RD, /WR           clk ─► NOT ─► VIA PHI2;  RW ─► VIA R/W

IRQ0–7 (active low) ─► 74HCT574 "snapshot" @ clk ─► 74HCT148 ─► /GS ─► NOT ─┬─ AND NOT VEC ─► INT
                                                       │                     │
INR ─► 74HCT74 @ clk ─► VEC (vector step) ─────────────┼─────────────────────┘
                                                       ▼
          VEC ─► 74HCT244: bus_addr A0–A7 = 1111 0 n2 n1 n0 (= 0xF0 + n, n = NOT 148 outputs)
          VEC ─► 74HCT541: bus_addr A8–A15 = 00
          VEC ─► 74HCT540: bus_data = NOT snapshot (bit k = 1: line k active) → register I
```

| Ports | Chip select | Chip | Registers |
|---|---|---|---|
| `0x00–0x0F` | /CS0 | W65C22N VIA | 16 (RS0–RS3 = A0–A3) |
| `0x10–0x17` (`0x18–0x1F` mirror) | /CS1 | 16C550 UART | 8 (A0–A2) |
| `0x20–0x2F` | /CS2 | emulator-only virtual monitor, until the real video exists (Q6) | 5 used |
| `0x30–0x7F` | /CS3–/CS7 | free: 2nd VIA, video, mask register… | — |
| `0x80–0xFF` | none (A7 on /G2A) | nothing | — |

Interrupt lines (vector = `0xFE00 + 2 × (0xF0 + n)` = `0xFFE0 + 2n`):

| Line n | Number | Vector at | Source |
|---|---|---|---|
| 7 (highest) | `0xF7` | `0xFFEE` | UART INTR (inverted) |
| 6 | `0xF6` | `0xFFEC` | VIA /IRQ (open drain: pull-up needed) |
| 5–1 | `0xF5–0xF1` | `0xFFEA–0xFFE2` | free (2nd VIA, video vblank, …) |
| 0 | `0xF0` | `0xFFE0` | unconnected: spurious / "no line" (F12) |

Vector numbers `0x00–0xEF` and `0xF8–0xFF` are left to software `INT n` (system calls). Software can also run `INT 0xF7` to call a hardware handler on purpose, for tests.

**Notes on the chips**

- **74HCT574 snapshot** plus a **74HCT74** (VEC, and a spare flip-flop). They make the IOD synchronous, as I2 / I5 require. Rough timing at 25 °C, to check with `timing.py`: INT ≈ 574 + 148 + 2 gates ≈ 110 ns (< 150, I5). Vector address ≈ 74 + 244 enable, or 574 + 148 + 04 + 244 ≈ 130 ns (< 237, I6). Data ≈ 74 + 540 enable ≈ 70 ns (< 335).
- **W65C22N**: `N` = open-drain /IRQ (correct), so it needs a pull-up (about 3.3 kΩ). The `S` version (totem pole) works just as well behind the snapshot. Its timers count PHI2 = 2 MHz, so T1 tops out at 32.8 ms (a 100 Hz tick = 20,000 counts). Known shift-register quirks (shift-in with an external clock, the SPI mode) are a reason to decide PS/2 and SD later; many builds bit-bang SPI on port B.
- **16C550**: DIP-40 (TL16C550CN, ST16C550CP40), with 1.8432 MHz crystal. ADS tied low, the active-high RD / WR pins tied inactive, MR from `rst_n` inverted, INTR active high (inverted for the snapshot). Check the DIP stock when ordering. The **6850 fallback** is a 6800-bus chip (E + R/W, like the VIA), has **no FIFO and no baud generator** (it needs an external clock), and would need a slower baud rate.
- **74HCT00** gates: /WR = NAND(IE·NOT RW, NOT `clk_wr_n`), low 250–375 ns, so the UART captures at the rise, like I4. /RD: see Q2.
- **Optional 74HCT574 mask register**: not needed now. The VIA (IER) and the UART (IER) already mask each of their sources (Q12).
- **I9** (rail-to-rail outputs on both buses) is met: the 244 / 540 / 541 are HCT.

## 3. Existing peripherals: where each one goes

| Today | New place | New ports | Interrupt | Why |
|---|---|---|---|---|
| IOD port latches, queue, port 0 detection | **Removed** from the machine. Kept as `BenchIod` (renamed) for the CPU verification tools and tests only | — | — | Nothing to detect: the ports are fixed by wiring. The bench needs the stub (section 1). |
| Monitor | **Emulator-only virtual video card** behind /CS2, same 5 registers, marked "not buildable" in the docs (Q6) | `0x20` DATA_0, `0x21` DATA_1, `0x22` POS_X, `0x23` POS_Y, `0x24` CMD | none (as today) | Keeps `screen.has`, the monitor window and graphics programs working until the real video is designed. |
| Keyboard (scan codes) | **Replaced by the UART console**: the PC terminal's keyboard sends ASCII (plus ANSI sequences for arrows etc.) into RBR. **Comes back as hardware:** a PS/2 keyboard on the VIA (PS/2 clock on CB1, data into the shift register or a port B pin), as a new model (Q7). | UART `0x10–0x17` | 7 (UART) | That's what the hardware will have. |
| RTC | **Removed** for now. Time comes from the VIA's T1 tick (`clock()`, delays). **Comes back as hardware:** a **DS3231** RTC (Gianni, 2026-11-10) over I²C bit-banged on two VIA port A pins, its 1 Hz SQW on CA1 (Q8). | VIA `0x00–0x0F` | 6 (VIA) | No RTC on the I/O board yet. |
| EEPROM | **Removed**, along with the EEPROM build target (1 MiB image, `boot.has`). The **boot ROM** already holds the program. **Comes back as hardware:** an SD card over SPI on the VIA (port pins + the shift register), or a **24LC256** I²C EEPROM on the RTC's bus (Q9). | — | — | F9: it models no hardware. |
| (new) VIA | system tick (T1), T2, ports A / B (LEDs and switches panel in the IDE), SR | `0x00–0x0F` | 6 | |
| (new) UART | console | `0x10–0x17` | 7 | |

## 4. Emulator and IDE changes

### hbc2-core

- **Remove** from the machine: `Iod`, `Interrupt`, `PORT_COUNT` / owners / `connect`, the `Peripheral` trait's `DEVICE_ID` / `PORT_COUNT` / `first_port`, and `Peripherals::port_written` routing. **Keep** the latch + queue model as `BenchIod`, used by `emutrace`, `hbc2-ucode` check, and the CPU / cycle tests.
- **New `io` module** on `Board` (replacing `board.iod`):
  - `IoBus`: decodes the port (A7 = 1 or a free /CS → nothing). `read(port) -> u8` **with side effects** (`&mut`), `write(port, v)`, and `peek(port)` **without side effects** for the debugger. Reads from an empty port return `0xFF` (Q11).
  - `IrqController`: 8 lines, priority 7 down to 0, the snapshot taken each tick, `INT = any && !vec`, the vector step `address_bus = 0x00F0 + n`, `data_bus` = line bitmap. This replaces `Board::tick_iod`. `cpu.rs`'s handshake stays as it is, because INT drops in the vector step (F5).
  - `Via` (W65C22): ORA / ORB / DDRA / DDRB with pin inputs, T1 one-shot and free-run (PB7 output, latches, read T1C-L clears the flag), T2 one-shot and pulse counting, SR modes, ACR, PCR (CA1 / CA2 / CB1 / CB2 edges and handshakes), IFR / IER with the bit-7 rules. **Lazy time**: counters are computed from the machine's tick count when accessed, and the next IRQ is a scheduled tick, so the fast paths stay fast (`Machine::wait` batches, idle skipping).
  - `Uart16550`: RBR / THR, IER, IIR (priority: line status > RX data / timeout > THRE > modem), FCR (FIFO enable, reset, trigger 1 / 4 / 8 / 14), LCR (DLAB, word format), MCR (loopback), LSR, MSR, SCR, DLL / DLM. RX and TX FIFOs of 16, **paced at the baud rate** in CPU cycles (Q13). Host side: `serial_input(bytes)` and `take_serial_output()`.
  - `VirtualMonitor`: today's `Monitor` with fixed ports.
- **`cpu.rs`**: only `O::In` changes, to `board.io.read(port)` (needs `&mut`). `O::Out` calls `board.io.write`. `enter_interrupt`, the vector table and the cycle counts don't change.
- **`machine.rs`**: `MachineConfig` becomes `{ virtual_monitor: bool }` (the VIA and UART are always there). `key_event`, `sync_rtc`, `rtc_interrupt` and `advance_time` are removed. `serial_input` / `serial_output` and `via_pins` are added. `is_idle` and `run` must count the next scheduled device event (a VIA timeout, a UART byte finishing): skip up to it, not to the end of the budget.
- **Trace / probe**: add `io_read(port, value)` next to `io_write`, since reads now matter.

### hbc2-runner

- `worker.rs`: remove `sync_rtc` and `key_event` forwarding. Add serial in/out channels (keystrokes in, bytes out, batched per frame). `lib.rs`: drop `sends_rtc_interrupts`. At slow speeds, the VIA tick and the baud rate slow down with the emulated clock, as they would on the hardware.

### hbc2-ide

- **New Terminal window**: a VT100 / ANSI subset (cursor moves, clear, 16 colors, scrolling) that shows UART output and sends typed keys to RX, 80 × 24. It replaces the keyboard capture of the monitor window (`keymap.rs` scan codes go; Enter / Backspace / arrows map to ASCII / ANSI). Paste is paced by the UART. Later, an option bridges it to a real COM port, so the PC terminal and the IDE work the same.
- **I/O view (`views/io.rs`)**: replace the IOD sections (port table with owners, interrupt queue, plug list, "RTC interrupt now" button) with: the INT / INR / VEC lines; the 8 IRQ lines with their source, which one is active and which one is being served; the port map `0x00–0x7F` with register names and `peek` values; a **VIA panel** (registers, live T1 / T2 counts, IFR / IER bits, PA / PB pins with LEDs and switches); a **UART panel** (registers, FIFO contents, baud rate and format, LSR bits).
- **Monitor window (`views/monitor.rs`)**: shown only when the virtual monitor is on. It no longer takes keys.
- **Menus and commands (`chrome.rs`, `app.rs`)**: remove `SaveEeprom` and `RtcInterrupt`. **`emulator.rs`**: remove `Image::Eeprom`, `Target::Eeprom` and the EEPROM breakpoint ranges; the peripheral checkboxes become "virtual monitor". **`settings.rs`**: the monitor-window settings stay, terminal window settings are added.
- **Memory view**: label `0xFFE0–0xFFEF` as the hardware vectors.

## 5. Code shipped with projects, library, tools and docs

| File | Uses | Change |
|---|---|---|
| `hbc2/templates/interruptHandler.has` | port 0 detection, device IDs, `monitorPort`…, RTC vector patched at `FE00 + 2p`, I = port / ID | **Rewritten**: `.define`s for ports and registers (VIA_ORB… UART_RBR…), and vectors `$0xFFE0–$0xFFEF` filled in. Line 0 → `irt`; a UART handler skeleton (read IIR, RBR into a ring buffer, `irt`); a VIA handler skeleton (read IFR, clear T1 with T1C-L, count ticks). Plus `io_init` (UART 38400 8N1 with FIFO, VIA T1 at 100 Hz, enable both IERs). Comments say "clear the source before IRT". Maybe split the defines into a second include, `io.has`. |
| `hbc2/templates/main.has` | includes the handler | `cal io_init`, then a "Hello" over the UART instead of `nop` loops. |
| `hbc2/templates/main.c` | comment: "once the monitor, the clock and the keyboard are found" | Comment updated (console = serial terminal). |
| `compiler/runtime/devices.has` | the whole port-0 detection | **Deleted.** |
| `compiler/runtime/crt0.has` | device variables, includes devices.has | Device variables removed. Calls `io_init` (vectors, UART, tick). |
| `compiler/runtime/init.has` | waits for detection (`CRT0_DEVICE_WAIT`) | No wait. |
| `compiler/runtime/boot.has` | EEPROM boot loader, `_eeprom_port` at `0x01FF` | **Deleted** with the EEPROM target. |
| `compiler/lib/include/hbc2.h` | device IDs, `*_PORTS_COUNT`, `*_port` externs, `set_vector(port, …)` (refuses 0), RTC, EEPROM, keyboard sections | IDs, counts and port variables go. `in` / `out` stay. `set_vector(number, handler)`: any number, with `IRQ_UART` / `IRQ_VIA` / `IRQ_VECTOR(n)` constants. VIA / UART register `#define`s. RTC and EEPROM sections removed (Q8, Q9). `key_event` replaced by `serial_getc_nowait` / `kbhit`. Monitor section kept (virtual monitor, fixed ports). New `ticks()` (VIA tick count). |
| `compiler/lib/asm/hbc2.has` (30 IN / OUT) | `set_vector` (port 0 refused), RTC, EEPROM, `srand_rtc`, key events | `set_vector` without the port-0 rule. RTC and EEPROM functions removed. `srand_rtc` → `srand_ticks` (T1 counter). |
| `compiler/lib/asm/screen.has` (46) | `_monitor_port` + offsets | Fixed ports `0x20–0x24`. A "no monitor" check reads a status (Q6). |
| `compiler/lib/asm/terminal.has` (11) | monitor scrolling, keyboard vectors at `FE00 + 2p`, RTC seconds for the cursor blink, scan codes in I | **Rewritten over the UART**: `putchar` → THR (waits on LSR THRE, or a TX ring buffer), `term_clear` / colors / cursor → ANSI sequences (CGA order → ANSI color table), `term_readline` from the RX ring buffer filled by the UART handler, no cursor blink (the terminal does it). |
| `compiler/lib/asm/time.has` (7) | RTC ports | `time()` = a software clock: seconds counted from the VIA tick, settable with `stime`. `clock()` = ticks, no longer -1. A real RTC later (Q8). |
| `compiler/lib/asm/math/misc/random.has` (5) | RTC ports for `srand_rtc` | Seed from T1. |
| `compiler/lib/asm/stdio.has` | goes through terminal.has | No change to the API. |
| `compiler/tests/hardware.c`, `eeprom.c`, `multi/vector/` (keyboard vector), `time.c`, `random.c` + the `.screen` / `.keys` files | old devices | `hardware.c` rewritten for VIA / UART. `eeprom.c` deleted. `multi/vector` uses `IRQ_UART`. `programs.rs`: `.keys` → `.serial_in`, add `.serial_out` (expected console output), keep `.screen` for the virtual monitor. |
| `hbc2-asm` (`assembler.rs` 677–774, 1075; `lib.rs`) | `IVT_START` as the end of program space; the EEPROM target's copied ranges | No change for the vector table: there's no special handling beyond `IVT_START`, and vectors stay `.data` at fixed addresses. **Remove `Target::Eeprom`.** No new built-ins (ISA-stays-basic rule): names come from `io.has`. |
| `hbc2-cc` (`lib.rs`, tests) | EEPROM target, boot.has size check | Remove the EEPROM target. |
| `hbc2-cli` (`main.rs`) | EEPROM image options | Remove them. `boot-rom` export stays. |
| `Documentation/Peripherals/` (README + 4 device pages + Specifications.txt) | the whole IOD model | **Replaced** by an I/O reference: port map, interrupt lines and vectors, VIA page, UART page, virtual monitor page (the old Monitor page, ports fixed). Keyboard, RTC and EEPROM pages are deleted. |
| `Documentation/HBC-2 Assembly Language.md` | opening example (monitor on ports 1–5), line 223 (`$0xFE02`, "port 1's vector"), 417 (`interruptHandler.has`), 431–440 (EEPROM target), 471 ("port n: `$0xFE00 + 2n`") | UART hello example. Vector example `$0xFFEE`. "Interrupt n: `$0xFE00 + 2n`; hardware lines are n = `0xF0–0xF7`". EEPROM section removed. |
| `Documentation/src/HBC-2_IDE_Instruction_Manual.odt` → `HBC-2_Instruction_Manual.pdf` | "256 ports (port #0 reserved for CPU <> IOD)", IOD × 15, queue × 5, INR × 3 | I/O chapter rewritten: port space, the handshake with the snapshot / VEC, the vector is `0xF0 + n`, I = line bitmap, "clear the source before IRT". The CPU instruction pages don't change. |
| `Documentation/HBC-2_Input_Output.pdf` (the new sketch, in French) | 244 enabled by INR, A0–A7 only, INT straight from /GS | Fix F1–F4 (VEC flip-flop, high byte, data driver, snapshot). |
| `hbc2/README.md`, `compiler/ABI.md` §8 (startup) and §10 (handlers), `compiler/ROADMAP.md` step 13, `compiler/lib/README.md`, `TODO.md` (RTC lines) | describe detection, EEPROM, IOD | Updated. |
| `logisim/notes/iod-requirements.md` | contract | Target chips added (done with this proposal, marked "proposed"). I1–I9 unchanged. |
| `logisim/tests/cpu/*.has`, the bench stub | bench IOD | **No change**: the CPU and its bench stay. |

## 5b. Compatibility

**Recommendation: clean break.** Tag the last IOD commit (`iod-legacy`) so old programs can still be built and run from that tag.

A "legacy IOD" machine option would be cheap in `hbc2-core` (the code exists and stays as `BenchIod`). But the cost is elsewhere: two interrupt templates, two runtimes (`devices.has` vs `io_init`), two C libraries (`terminal.has` over the monitor vs over the UART), two sets of docs, and IDE views for both. Old programs depend on the parts that change most: port-0 detection, ports given at power-on, latches that read back, interrupt number = port, data in I, scan codes. The only programs that exist are the templates, examples and tests in this repo, and they all get ported in this change.

## 6. Open questions (with recommendations)

| # | Question | Recommendation |
|---|---|---|
| Q1 | **VIA bus timing.** | **Checked 2026-11-10: fails** with any PHI2 the clock generator can give (section 8). |
| Q2 | **/RD window for the UART.** | **Checked 2026-11-10: fails** at both ends of the window (section 8). |
| Q3 | Data in the vector step: `0x00` or the IRQ states? | **IRQ states** (bitmap, 1 = active) through a 74HCT540 from the snapshot. Same chip count as 0x00 (a driver is needed anyway, F3). The handler can see other pending lines and serve them before IRT, which saves a 9-cycle re-entry, and one shared handler becomes possible. |
| Q4 | A7: mirror or not? | Put A7 on the 138's /G2A (free). `0x80–0xFF` stay empty instead of mirroring, and are left for a second decoder. |
| Q5 | Line numbers. | UART 7, VIA 6 (as proposed: the UART RX overruns first). Line 0 stays unconnected, so spurious entries (n = 0) land on a plain `irt`. |
| Q6 | Keep the Monitor? | Yes, as an **emulator-only virtual video card** on /CS2 (`0x20–0x24`), clearly marked not buildable, until the real video is designed. Add a read-only status / ID register at `0x25`, so `screen.has` can tell it's there (an empty port reads `0xFF`). |
| Q7 | Keyboard. | Console keys come through the UART (ASCII / ANSI). The scan-code keyboard and `key_event` go. A PS/2 model on the VIA comes later, with the PS/2 hardware. |
| Q8 | Real-time clock. | Now: a software clock from the VIA tick. Later: a DS3231 (I²C bit-banged on PA, SQW 1 Hz → CA1; Gianni, 2026-11-10). The emulator then models it behind the VIA pins and can still sync it to host time. |
| Q9 | EEPROM. | Remove it and its build target. The boot ROM covers power-on. Storage later: an SD card over SPI on the VIA, or a 24LC256 on the RTC's I²C bus. |
| Q10 | Baud rate and terminal type. | 38400 8N1 (divisor 3 at 1.8432 MHz), FIFO trigger 8. ANSI / VT100 subset, the same in the IDE terminal and on the PC (PuTTY, Tera Term). |
| Q11 | IN from a port with no chip. | Undefined on the hardware (floating bus). The emulator returns `0xFF`, and the docs say "don't rely on it". No pull-ups on the CPU board. |
| Q12 | Mask register (74HCT574 on /CS3). | Not now: VIA and UART IERs mask each source. Add it when a source without its own enable shows up. |
| Q13 | UART timing in the emulator. | Pace bytes at the baud rate (so THRE / FIFO-full handling is tested like on the hardware). An "instant serial" IDE option can come later if pasting large texts is too slow. |
| Q14 | Legacy option. | No (section 5b). |
| Q15 | Vector `0xF0 + n` needs the 148's outputs inverted. Without the inverters it would be `0xF7 − n`. | Keep the inverters and `0xF0 + n`: it matches the docs, and three gates cost little. |

## 7. Implementation order

1. **Review this proposal** and answer Q1–Q15. Fix the sketch PDF (F1–F4).
2. **Hardware timing on paper**: Q1 (VIA write hold) and Q2 (/RD window) with `timing.py` and the datasheets. These are the only answers that could change the chip choice, so they come before any code. **Done 2026-11-10: both fail (section 8); waiting for Gianni's choice of option.**
3. **hbc2-core**: rename `Iod` → `BenchIod` (emutrace, ucode check, CPU / cycle tests pass unchanged). Add `IrqController` + `IoBus`, then `Uart16550`, `Via`, and the fixed-port `VirtualMonitor`. Lazy device time in `Machine::run` / `is_idle`. Tests: priority and vector numbers, INT dropping in the vector step, re-entry after IRT when the source isn't cleared, I = bitmap, VIA timers / IFR rules, UART IIR priority / FIFO / pacing, `peek` has no side effects.
4. **Remove the old devices**: RTC, keyboard and EEPROM models, the EEPROM target in `hbc2-asm` / `hbc2-cc` / `hbc2-cli` / IDE.
5. **Library and runtime**: `devices.has` and `boot.has` out. `crt0` / `init` with `io_init`. `hbc2.h` / `hbc2.has`, `terminal.has` over the UART, `screen.has` on fixed ports, `time.has`, `random.has`. C tests with `.serial_in` / `.serial_out`.
6. **Runner and IDE**: the Terminal window, the I/O view with the VIA / UART panels, menus and settings.
7. **Templates and docs**: `interruptHandler.has` (+ `io.has`), `main.has`, `main.c`, the I/O reference replacing `Peripherals/`, Assembly Language.md, the Instruction Manual (.odt → .pdf), READMEs, ABI.md, ROADMAP / TODO. Tag `iod-legacy` before merging.
8. **Logisim `iod` module** (decoder, snapshot, VEC, vector drivers, glue; the VIA and UART as pin stubs), tested against the emulator's `IrqController`. Then the breadboard card.
9. **Later**: PS/2 keyboard, DS3231, SD card or 24LC256, 2nd VIA, the real video.

## 8. Step 1 result: Q1 and Q2 both fail (2026-11-10)

On 2026-11-10 Gianni accepted the corrections (F1–F12) and the recommendations, and asked for Q1 and Q2 to be settled first, stopping if either fails. **Both fail.** The cause is the CPU's I/O step, not the choice of VIA or UART. Nothing has been built, and the emulator is unchanged.

### 8.1 What the CPU gives an I/O chip

Times are in ns from the `clk` edge that starts the IN / OUT step, at max 25 °C and 50 % duty (`python tools/timing.py --section cu`).

| What | When | Source |
|---|---|---|
| `bus_addr` = 00:port valid | **72** | The latch loads the port at the start of the I/O step itself: `hbc2.uc` `instr in` = `lat<-0:b`, then `bus<-io, r1<-bus, lat<-pc`. |
| 74HCT138 outputs settled (glitches before) | **108** | 72 + A → Y 36 |
| IN: data on `bus_data` valid by | **335** (322 at 45 % duty) | earliest 670 /WE rise 347 − setup 12 (C10, I3) |
| IN: data held until | **428** (440.5 at 55 %) | latest 670 /WE rise + hold 5 |
| OUT: data valid / held | 242, held **to the edge (500)** only | I4. No minimum delay after the edge can be shown on paper (TI gives none for the 574, B1). |
| Address, IE and RW held | to the edge (500) | The IN / OUT step itself does `lat<-pc`. |
| Clock edges the IOD can use | osc falls at nominal 125 and 375. In the `clk` frame these are **97–125** and **347–375**, and up to 177 / 427 through the 10 and the 74 (Q2). `clk` falls at about 250 and rises at 500. | `timing.py` takes t = 0 as `clk` rising, which can lag osc by FF1's 28 ns. |

So the only strobe edges are near 125, 250, 375 and 500 ns, and each can move by tens of ns. Two windows are needed: one from 108 to 322 (read data), and one from 428 to 500 (end of the access).

### 8.2 Q1: W65C22N fails

From the WDC W65C22 datasheet (table 4-3a, 5 V, 14 MHz grade):
- **Read:** CS, RS and RWB set up 10 ns before PHI2 rises (tACR). Data is valid 20 ns after PHI2 rises (tCDR) and held 10 ns after PHI2 falls (tHR).
- **Write:** data set up 10 ns before PHI2 falls and held 10 ns after (tDCW, tHW).
- **CS, RS and RWB hold:** 10 ns. WDC's table says this is from PHI2's rising edge, but the timing diagrams and the 6522 family use the falling edge.
- **Outputs:** IOL 3.2 mA, IOH −0.2 mA at 2.4 V (TTL levels).

The timers need PHI2 to run all the time, so there is a PHI2 pulse in every step, and the chip is accessed while PHI2 is high.
- **Read:** the data goes away 10 ns after PHI2 falls, and the register write needs it until 428. So PHI2 must fall at **418 or later**.
- **Write, and RS / CS / R/W hold:** the CPU only holds these to 500, and the chip needs 10 ns after PHI2 falls. So PHI2 must fall at **490 or earlier**.
- **Available falls:**
  - PHI2 = NOT `clk` falls at 500 or later (the proposal's choice). The write and address holds then rely on CPU delays after the edge, which breaks I4.
  - PHI2 = Q2 (or NOT `clk_wr_n`) falls at 347–427. The read data can be gone before the register write.
  - No edge falls surely between 418 and 490. **Fails.**

### 8.3 Q2: TL16C550C fails

From the TI datasheet (SLLS177I, 2021):
- **Read:** data is valid 45 ns after /RD falls (tRVD, at 75 pF). It floats at most 20 ns after /RD rises (tHZ), and **no minimum data hold** is given.
- **Address and CS:** set up 7 ns before /RD falls (tAR, tCSR). Held 20 ns (address, tRA) and 10 ns (CS, tRCS) after /RD rises.
- **Pulses and writes:** /RD and /WR pulses are at least 40 ns. Write data is set up 15 ns and held 5 ns (tDS, tDH).
- **Outputs:** IOL 1.6 mA, IOH −1 mA.
- **Package:** the DIP-40 (N) package is NRND.

- **Start.** Reading RBR, IIR or LSR has side effects, so /RD must not fall before the decode settles (108 + 7 = 115).
  - The osc phase at nominal 125 can come as early as 97, so a glitch could read a register.
  - The first edge that is surely later is `clk` falling at about 250. Then /RD falls at about 274 (one gate), and the data comes at about 319, even at the datasheet's 75 pF.
  - On the real `bus_data` (410 pF, driven at 1.6 mA) that is well past 322. **Fails.**
- **End.** The data isn't held after /RD rises, but the register needs it until 428. The address must stay 20 ns after /RD rises, but the CPU only holds it to 500.
  - So /RD must rise between 428 and 480, and no edge falls there. **Fails.**
  - The 16C550's ADS pin can latch the address and CS, which solves the address hold. It doesn't help the start or the deadline.

Both chips drive `bus_data` (about 410 pF) with weak outputs (1.6 / 3.2 mA), so a 74HCT245 between the I/O chips and `bus_data` would probably be needed in any case.

### 8.4 Why, and what could pass

The I/O step is one 500 ns step with no WAIT. The port appears at 72 ns, the register captures at 347–428 ns, and the address may change at 500 ns. That fits latches, which is why I7 says a real IOD answers "through hardware latches and a FIFO". Any bus chip whose strobe must frame the access hits the same two windows. The 82C55 + 82C54 fallback is not checked yet, but it faces the same windows on the same bus.

| Option | CPU | IOD | On paper |
|---|---|---|---|
| **A. Longer I/O cycle (microcode only)** | IN becomes a setup step, an access step and a hold step, with IE, RW and the port held through all three (`lat<-pc` only after). OUT becomes a setup step and an access step. Cycle counts change (IN +2, OUT +1), so the isa.rs table, cu-cycles.md and the emulator change too. | A 74HCT74 delays IE twice, so the strobe opens only in the access step, where the decode has been stable since the setup step. | **16C550: passes with large margins** (to be confirmed in `timing.py`). /RD opens about 70 ns into the access step and stays low to its end, so the data comes at about 170 plus the bus load. The hold step keeps the address. **Intel-bus chips** (82C55, 82C54) fit the same way, also to be checked. **W65C22: no clean wiring found even then.** A continuous PHI2 gives a high phase in every step, so CS must frame exactly one PHI2 pulse, which again needs edges the clock generator doesn't have. |
| B. CPU unchanged, buffered I/O bus | Q2 added to the connector (I1) | A 74HCT573 read latch, a 74HCT541 write buffer, a 74ACT138 decode (3–12 ns, settled by about 84) and strobes from Q2 | **Still fails on paper.** The 573 needs 10 ns data hold after LE, the UART gives none after /RD rises, and both come from the same phase. At 45 % duty the decode settles only 0.5 ns before Q2 can rise. Only a breadboard test could show it works. Not recommended. |
| C. CPU unchanged, an I7 IOD | none | Latches and a FIFO, filled by a microcontroller | Passes (I7). Not the standard-chips design. |

**Recommendation: option A, with the 16C550, and 82C55 + 82C54 instead of the VIA**, after checking their datasheets against the longer cycle. The VIA could only stay with a read latch, which is option B's weak point.
- **Needs Gianni's decision:** A changes the microcode and the cycle counts, and the brief says "the CPU must not change".
- **What 82C55 + 82C54 changes for the later hardware:** PS/2, I²C and SPI become bit-banged on 82C55 port pins. The PS/2 clock interrupt then uses the mode-1 strobe (PC4 /STBA → INTRA), because the 82C55 has no CA1 or shift register.

**Stopped here (step 1), as asked.** Steps 2–4 wait for the choice.

## 9. Option C: a mailbox and an ESP32 I/O processor (proposal, 2026-11-10)

Gianni chose option C on 2026-11-10. The CPU and its microcode don't change, and the peripherals live in a microcontroller behind a **mailbox** made only of latches and flip-flops. Sections 2–5 describe the VIA / UART design and are **superseded** wherever this section differs. Section 9.5 says what stays from them. Nothing is built yet: no Logisim, firmware or emulator change.

The design has one key choice:

**The ESP32 runs today's IOD model as its protocol.** The ESP32 firmware keeps:
- the 256 port latches;
- the peripherals with the same device IDs, ports and commands;
- the port-0 plug-in interrupts;
- the interrupt queue of (port, data).

The CPU reaches all of it through the mailbox:
- **OUT p, v** becomes a 3-byte command.
- **IN p** becomes a 2-byte command plus a 1-byte reply.
- **Interrupts reach the CPU exactly as today:** in the vector step the mailbox puts `00:p` on `bus_addr` and the data on `bus_data`, which loads register I.

The emulator then keeps `iod.rs` and the peripherals almost as they are. The C API (`in()`, `out()`, `set_vector()`, `monitor_port`…), `interruptHandler.has` and `devices.has` keep working. Only the lowest layer (the IN / OUT instructions in the library) changes. Native commands (strings, graphics primitives, SD blocks) come on top, for speed.

### 9.1 Mailbox hardware

#### Ports

There is one decoder. The mailbox sits at `0x00–0x0F`, with A1–A3 ignored (mirrors). `0x10–0x1F` is a second select, free for a second I/O processor (Q-C9). `0x20–0xFF` are empty.

| Port | IN | OUT |
|---|---|---|
| `0x00` | **RX**: the byte the MCU left for the CPU (reply channel). Reading it empties RX, but only if RX was full: a read of an empty RX gives a stale byte and changes nothing. | **TX**: a byte for the MCU. Ignored if TX is still full (like a UART overrun). |
| `0x01` | **STATUS**: bit 0 `RX_AVAIL` (1 = a byte waits in RX), bit 1 `TX_FREE` (1 = TX can take a byte), bit 2 `EVT_PEND` (an event is waiting to interrupt), bit 3 `READY` (the MCU has started since the last CPU reset), bits 4–7 = 0 | (free) |
| vector step | `bus_addr` = `00:p`, `bus_data` = d → register I, from the **event latches** | — |

The bits are set to 1 when the CPU can go ahead, so a polling loop is `in` / `and` / `jmz` (the ISA only has jumps on a set flag).

#### Three one-way channels

| Channel | Latch | Written by | Read by | Flag | Flag set | Flag cleared |
|---|---|---|---|---|---|---|
| TX (CPU → MCU) | 74HCT165 | the CPU (OUT `0x00` loads it in parallel from `bus_data`) | the MCU, shifted out over SPI | `tx_full` | CPU write | MCU ack |
| RX (MCU → CPU) | 74AHCT595 | the MCU (shifted in, latched by RCLK) | the CPU (IN `0x00`, through its /OE) | `rx_full` | MCU write | CPU read, at the end of the IN step |
| EVT (MCU → CPU, interrupt) | 2 × 74AHCT595 (port p, data d) | the MCU | the CPU, in the vector step | `evt_pend` | MCU write | the end of the vector step |

**Each flag is the XOR of two toggle flip-flops, one per side.** Each flip-flop is clocked by only one side, so no edge from one side can race a set or clear from the other. A side writes only when the flag says the latch is its turn:
- The MCU writes RX or EVT only when its flag is 0.
- The CPU writes TX only when `TX_FREE` is 1, and reads RX when `RX_AVAIL` is 1.

**A latch is never written while it is being read.** The CPU's toggles move at the end of its step (`clk`), after the register write at ≤ 428 ns, so the data is held for the whole read.

#### Logic

| Signal | How |
|---|---|
| Decode | 74HCT138: G1 = IE, /G2A = A7, /G2B = A5 OR A6, inputs A = A0, B = RW, C = A4. Y0 = OUT TX, Y2 = IN RX, Y3 = IN STATUS. Y4–Y7 = the second select (`0x10–0x1F`). |
| TX strobe `tx_wr_n` | OR(/Y0, `tx_full_s`, `clk_wr_n`) (2 × 74HCT32 gates, `clk_wr_n` last): low about 250–375 ns, only in an OUT `0x00` step with TX empty. Its falling edge loads the 165 (/PL), its rising edge toggles `cpu_tx` (74HCT74, D = /Q). |
| CPU toggles on `clk` (74HCT175 A, cleared by `rst_n`) | `cpu_rx` D = Q XOR (IN RX AND `rx_full_s`), with NOR(/Y2, /`rx_full_s`) on a 74HCT02. `cpu_evt` D = Q XOR VEC. **VEC** D = INR (I6: high in the vector step only). |
| MCU toggles (74HCT74, clocked on RCLK's fall) | `mcu_rx`, `mcu_evt`, `mcu_tx` (the ack): D = Q XOR the matching request bit of the CTRL 595. `ready`: D = Q OR READY_REQ, cleared by `rst_n`. |
| Flags | `rx_full` = `mcu_rx` XOR `cpu_rx`, `tx_full` = `cpu_tx` XOR `mcu_tx`, `evt_pend` = `mcu_evt` XOR `cpu_evt` (74HCT86). |
| Synchroniser (74HCT175 B, `clk`) | `rx_full_s`, `tx_full_s`, `evt_pend_s`, `ready_s`: the MCU-side flags change at any time. The CPU sees them through one `clk` stage, so they are stable for the whole step (STATUS, the TX gate, the RX toggle, INT). |
| **INT** | NOR(/`evt_pend_s`, VEC) (74HCT02): from a flip-flop on `clk` (I5), low in the vector step (I6, F5). The flag clears at the end of the vector step, so the next event can rise INT from step n + 2 at the earliest (I6). |
| Vector step | /VEC (175 /Q) enables: the EVT port 595 onto `bus_addr` A0–A7, a 74HCT541 with inputs tied low onto A8–A15 (F2), and the EVT data 595 onto `bus_data` (→ I, F3). The CPU's latch outputs are off then (C12). |
| IN RX / STATUS | /Y2 → the RX 595's /OE. /Y3 → the 74HCT244's /OE (inputs = the four `_s` flags, set to 1 when ready, bits 4–7 tied low). |
| Reset | `rst_n` clears every toggle, so every flag = 0: RX empty, TX free, no event, READY = 0. The MCU sees `rst_n` (level-shifted, below), resets its protocol state, sends the plug-in events again, then sets READY. |

#### Chip list (CPU side, one card)

| Chip | Qty | Job |
|---|---|---|
| 74HCT138 | 1 | decode |
| 74HCT32 | 1 | A5 OR A6, TX strobe (2 gates), `ready` D (1) |
| 74HCT02 | 1 | RX toggle condition, INT (2 spare) |
| 74HCT86 | 3 | 2 CPU toggles, 3 MCU toggles, 3 flags, 1 RCLK inverter (3 spare) |
| 74HCT175 | 2 | CPU toggles + VEC; synchroniser |
| 74HCT74 | 3 | `cpu_tx`; `mcu_rx`, `mcu_tx`, `mcu_evt`, `ready` (1 spare) |
| SN74AHCT595N | 4 | RX data, EVT port, EVT data, CTRL (MCU requests, MISO enable). **TI makes no 74HCT595 in DIP** (test-rig.md). The AHCT part is pin-compatible with TTL-level inputs, and is already the rig's part. |
| CD74HCT165E | 1 | TX data |
| 74HCT244 | 1 | STATUS |
| 74HCT541 | 1 | vector step: A8–A15 = 00 |
| SN74LVC244AN (at 3.3 V) | 1 | 5 V → 3.3 V for the ESP32: MISO (its own /OE from CTRL, because the SD card shares MISO), `tx_full`, `rx_full`, `evt_pend`, `rst_n` |
| **Total** | **19** | about the size of the address section's largest cards |

The ESP32's 3.3 V outputs (SCK, MOSI, RCLK) drive HCT / AHCT inputs directly (VIH 2.0 V).

Simplification if pins allow (Q-C3): give each request its own GPIO pulse instead of the CTRL 595 and its toggle logic. That saves 1 × 595 and 1 × 86, but costs the ESP32 4 more outputs.

#### Timing against the CPU step

Figures are max 25 °C, the pass criterion. They use `timing.py`'s figures and the datasheet figures below. Deadlines are at 45 % duty (I3: 322 ns).
- **Bus loads:** a temporary copy of `timing.py` was run with the mailbox's loads: `bus_data` 1 in, 3 out, so 454 pF instead of 410; `bus_addr` 2 in, 2 out. **Every section still passes at 25 °C at 45 % duty.** The extra load costs +5 ns on `bus_data` drivers.
- **HCT figures are used for the 595.** SN74HCT595: RCLK → Q 40, OE → Q 35, disable 30, 6 mA. The AHCT part is faster.
- **165** (CD74HCT165): /PL pulse ≥ 20, A–H setup to /PL↑ 20.
- **244** (CD74HCT244): OE 30.

| Path | Max 25 °C | Limit | Margin |
|---|---|---|---|
| IN RX: A5 / A6 (72) → 32 (24) → 138 G → Y (33) → 595 OE → Q (35 + 41 load) | 205 | ≤ 322 (I3) | 117 |
| IN STATUS: the same → 244 OE (30 + 41) | 200 | ≤ 322 | 122 |
| STATUS data: 175 CLK → Q (33) → 244 A → Y (25 + 41) | 99 | ≤ 322 | 223 |
| TX strobe glitch-free: decode settled (72 + 24 + 33) before `clk_wr_n` can fall (≥ 222) | 129 | < 222 | 93 |
| OUT TX: `bus_data` valid (242 + 5 load) + 165 setup (20) before the earliest /PL rise (347) | 267 | ≤ 347 | 80 |
| OUT TX: latest /PL rise: `clk_wr_n` (375 + 24) → 32 (24) | 423 | ≤ 500 − 165 hold (not given by TI; typ. a few ns) | about 75 |
| TX strobe pulse, shortest (as the /WE pulse) | 36.5 at 45 % | ≥ 20 (/PL) | 16.5 |
| RX toggle: /Y2 (129) → 02 (21) → 86 (32) → 175 setup (20) | 202 | ≤ 500 | 298 |
| INT: 175 CLK → Q (33) → 02 (21) | 54 | ≤ 150 (I5) | 96 |
| Vector step, `bus_addr`: /VEC (33) → 595 OE (35 + 13 load) | 81 | ≤ 237 (I6, C12) | 156 |
| Vector step, A8–A15: /VEC (33) → 541 OE (30 + 13) | 76 | ≤ 237 | 161 |
| Vector step, `bus_data`: /VEC (33) → 595 OE (35 + 41) | 109 | ≤ 335 (I6) | 226 |
| **Drivers off after the edge**: IN = IE (33) → 138 G (33) → 595 / 244 disable (30); vector = 175 (33) → disable (30) | **96 / 63** | 50 (I3, I6) | **−46 / −13** |
| MCU write: data stable (RCLK↑ + 40) before the toggle (RCLK↓) | — | RCLK high ≥ 60 ns (firmware rule) | — |

All the strobes are clean:
- The only write strobe (TX) is gated by `clk_wr_n` after the decode has settled.
- The RX read has no side effect while the address settles. The flag toggle is sampled at the end of the step.
- Nothing needs a hold time from the CPU after the edge, except the 175s' D inputs. Those are the same class as every CPU flip-flop (B1).

**The only miss is "off by 50 ns after the edge" (I3, I6).** The mailbox's outputs turn off in up to 96 ns (IN) and 63 ns (vector step). But the CPU's own `bus_data` drivers don't meet 50 either: 670 output 33 + 35 = 68, RAM read buffer 33 + 30 = 63. The 50 ns figure looks like a stub assumption from ir-mem section 4, not a derived limit. Q-C1 proposes making I3 / I6 say "within the CPU's own driver turn-off, short overlaps accepted as C5 / registers-decisions D-notes", plus a breadboard check like B4. The IN path could get to 63 by moving IE off the 138's slowest input, but that costs a gate on the address path and doesn't remove the overlap.

#### MCU side

- **Write RX or EVT:**
  1. Shift 32 bits over SPI (SCK, MOSI) through the chain CTRL → EVT port → EVT data → RX. Shifting doesn't disturb the outputs.
  2. Pulse RCLK, high ≥ 60 ns.
  3. Every storage register takes the new contents at once: the MCU re-sends the current value of the latches it isn't changing.
  4. The CTRL bit `REQ_RX` / `REQ_EVT` toggles the MCU's flip-flop on RCLK's fall.
- **Read TX:**
  1. Set MISO_EN (CTRL) to enable the 165's clock and its MISO buffer.
  2. Shift 8 bits in.
  3. Write CTRL with `REQ_ACK` (clears `tx_full`) and MISO off.
  4. The SD card shares SCK / MOSI / MISO. MISO_EN off keeps the mailbox off the bus during SD transfers.
- **Flags:** `tx_full`, `rx_full`, `evt_pend` and `rst_n` reach four ESP32 inputs directly through the LVC244. Reading them is one GPIO register read, about 0.1 µs.
- **Polling vs GPIO interrupt: polling** in a task pinned to one core (FabGL's VGA uses I2S DMA and needs little CPU). An ESP32 GPIO interrupt costs about 2–3 µs of latency plus jitter (typical figures, to be measured). A polling loop on `tx_full` reacts in well under 1 µs and keeps the order of events simple.
- **Time per byte:** a bit-banged shift of 8 or 32 bits with direct register writes takes about 1–2 µs, or about the same with the SPI peripheral (to be measured). **About 2–4 µs per byte, against the CPU's 16–36 cycles (8–18 µs) per byte (9.2):** the CPU is the bottleneck.

### 9.2 Throughput and latency

The CPU runs at 2 MHz (`HARDWARE_CLOCK_HZ`), and the cycles come from isa.rs. IN and OUT take 4 cycles; `and` r, imm8 5; `jmz` Reg16 3; `lod` / `str` (RamReg) 6; `inc` / `dec` Reg 4; `cmp` Reg 3.

| Loop | Cycles per byte | Time | Rate |
|---|---|---|---|
| Send, MCU always ready: `in` STATUS, `and`, `jmz` (not taken), `out` TX | 16 | 8 µs | 125 kB/s ceiling |
| Send from a buffer: + `lod`, `inc` pointer, `dec` count, `jmz` end, `jmp` loop | about 36 | 18 µs | about 55 kB/s |
| Receive into a buffer: `in`, `and`, `jmz`, `in` RX, `str`, `inc`, `dec`, `jmz`, `jmp` | about 37 | 18.5 µs | about 54 kB/s |
| Event: entry (9) + a short handler (about 20) + `irt` (6) | about 35 | 17.5 µs | about 57 k events/s |

The ESP32 serves a byte in about 2–4 µs (9.1), so the CPU sets the pace. Every OUT or IN that goes through the protocol also masks interrupts (`psf`, `cli` … `pof`: about 7 cycles), so a handler's own commands can't split a command in the middle (9.3).

| Job | Compat commands (today's ports) | Native commands |
|---|---|---|
| One character on the monitor (WRITE: DATA_0, DATA_1, POS_X, POS_Y, CMD = 5 OUTs) | 5 × 3 bytes, about 15 × 18 = 270 µs, about 3,700 chars/s; a 42 × 24 screen in 0.27 s | `PUTS`: 1 byte per char, about 55,000 chars/s; an 80 × 25 screen in 37 ms |
| One pixel (same 5 OUTs) | about 270 µs, a 256 × 192 frame in 13 s (today's emulator: about 15 µs per pixel, 0.7 s) | `PIXEL` x, y, c: 4–5 bytes, about 80 µs. **Primitives** (`LINE`, `RECT`, `FILL`, `SPRITE`) cost a few bytes each, and the ESP32 draws them. `BLIT` of a 4-bit row: 2 pixels per byte, a full 256 × 192 frame (24 KiB) in about 0.45 s. |
| Key press → handler | PS/2 decode (FabGL) + an event: a few µs to the INT flip-flop, then the next instruction boundary + 9 cycles | same |
| `GET_TIME` round trip | 7 OUT / IN commands, about 0.3 ms | 1 + 6 bytes, about 0.15 ms |
| SD block (512 B) read | — | ESP32 SD read (about 1–2 ms) + 512 × 18.5 µs = about 11 ms, about 45 KiB/s |

**VGA text is fast enough. Per-pixel drawing through the old monitor ports is slow** (5 OUTs per pixel become 15 bytes). Graphics programs should use the native primitives. Keeping the monitor's port protocol is for compatibility, not speed.

### 9.3 Command protocol (first version)

A single byte stream runs CPU → MCU on TX. Replies come back on RX in order, and events come on EVT. A command is atomic: the library sends it with interrupts masked, so a handler's commands never interleave with the main program's. After a CPU reset (`rst_n`) the MCU drops any partial command.

**Compat commands, today's IOD exactly:**

| Byte | Command | Reply | What the MCU does (= `hbc2-core` today) |
|---|---|---|---|
| `0x01` p v | OUT | — | `Iod::cpu_write(p, v)`, then `Peripherals::port_written(p)` (monitor CMD, RTC CMD, EEPROM CMD…) |
| `0x02` p | IN | 1 byte | `Iod::port(p)` |

**Events.** Every `Iod::trigger_interrupt(p)` becomes an event (p, d) in the MCU's queue (256 entries, dropped when full, as now). The events are:
- the two port-0 plug-in interrupts (first port, then ID), sent after start-up and after each CPU reset;
- the RTC's one-second interrupt on its first port;
- the keyboard's pressed and released scan codes, on its two ports. The ESP32 maps PS/2 to the HBC-2 scan codes of `keymap.rs` (AZERTY, ASCII-like).

Port assignment stays dynamic, in the same order (monitor 1–5, RTC 6–12, keyboard 13–14, EEPROM 15–19). `devices.has` and `interruptHandler.has` work unchanged.

**Native commands (v1, added next to compat):**

| Byte | Command | Arguments | Reply |
|---|---|---|---|
| `0x00` | NOP | — | — |
| `0x03` | INFO | — | protocol version, firmware version, devices bitmap (3 bytes) |
| `0x10` | PUTS | n, n bytes (n = 0 → 256) | — (ANSI / VT100 subset on a FabGL text terminal; or into the monitor's text mode while it is shown, Q-C5) |
| `0x11` | CURSOR | x, y | — |
| `0x12` | COLORS | fg / bg (nibbles) | — |
| `0x13` | CLEAR | — | — |
| `0x20` | PIXEL | x lo, x hi, y, c | — |
| `0x21` | LINE / `0x22` RECT / `0x23` FILL | x0, x0 hi, y0, x1, x1 hi, y1, c | — |
| `0x24` | BLIT | x, x hi, y, n, n / 2 bytes (2 pixels per byte) | — |
| `0x25` | MODE | 0 = monitor 256 × 192 (today's), 1 = text 80 × 25, 2 = 320 × 200 × 16 | — |
| `0x30` | GET_TIME | — | 6 bytes: year − 1900, month, day, hour, minute, second (the RTC's port layout) |
| `0x31` | SET_TIME | 6 bytes | 1 (ok) / 0 (invalid), as the RTC's SET_TIME |
| `0x40` | SD_READ | 4-byte block number | status, then 512 bytes if status = 0 |
| `0x41` | SD_WRITE | 4-byte block number, 512 bytes | status |
| `0x50` | SOUND | frequency (2 bytes), duration (×10 ms), volume | — |

**Storage:** the EEPROM device (1 MiB, ID `0xE1`, same ports) stays as a compat device backed by a file `eeprom.bin` on the SD card. **Its power-on copy into RAM goes** (F9): the ESP32 can't write the CPU's RAM, and the boot ROM loads programs. SD_READ / SD_WRITE give raw blocks, and a FAT library on the CPU side can come later.

### 9.4 ESP32 board and pins

**Candidate: LilyGO TTGO VGA32 v1.4** (ESP32-WROVER with PSRAM, VGA 6-bit RGB resistor DAC, two PS/2 ports, micro-SD, audio). It's FabGL's reference board. The pin use below is from FabGL's configuration for this board and **must be checked against the board's schematic (vga32_v1.4.pdf) before any wiring**:

| Use | GPIO |
|---|---|
| VGA R1 R0, G1 G0, B1 B0, HSYNC, VSYNC | 21 22, 18 19, 4 5, 23, 15 |
| PS/2 keyboard (CLK, DAT) | 33, 32 |
| PS/2 mouse (CLK, DAT) | 26, 27 |
| SD (SPI: MISO, MOSI, SCK, CS) | 2, 12, 14, 13 |
| Audio | 25 |
| PSRAM (WROVER) | 16, 17 |
| Strapping / USB serial | 0, 1, 3 (and 2, 12, 15 are strapping too) |

**The mailbox needs, with the CTRL 595 design:**
- SCK, MOSI and MISO, shared with the SD card (14, 12, 2);
- **1 output**, RCLK;
- **4 inputs**: `tx_full`, `rx_full`, `evt_pend`, `rst_n`. The input-only GPIOs 34, 35, 36 and 39 are the natural place. One source says IO36 is a button on this board, to confirm.

RCLK could take 26 or 27, giving up the mouse port, or GPIO 0, which is fine once booted if nothing pulls it low at reset. So **it fits on one TTGO VGA32 without the mouse**, if 34 / 35 / 39 and one more input are free on the schematic.

**Fallback if the pins don't fit:** a bare ESP32-WROVER DevKit on the I/O card, with the TTGO's VGA resistor DAC, a PS/2 socket and a micro-SD socket copied from its schematic. That has enough free GPIOs, even for the 4-pulse variant without the CTRL 595.

**Not recommended for now:** a second MCU (an Arduino Nano on `0x10–0x1F`). One ESP32 covers VGA, PS/2, SD, Wi-Fi and sound. The decode leaves the select free for later (Q-C9).

**Wi-Fi:** NTP is possible, but Wi-Fi with FabGL's VGA uses a lot of RAM and can disturb the display. Recommendation: NTP once at start-up, then Wi-Fi off. A DS3231 on I²C (two free GPIOs, or the mouse port's) keeps the time without Wi-Fi (Q-C6).

### 9.5 Emulator, IDE and shipped code (replaces sections 3–5 for option C)

**Most of the current model stays.**

| Part | Option C |
|---|---|
| `iod.rs` (`Iod`: latches, owners, queue, port-0 plug-in) | **Kept as is.** It becomes the ESP32 firmware model's state. It is still the bench IOD for emutrace, `hbc2-ucode` check and the CPU / cycle tests (no rename needed). |
| `peripherals/` (Monitor, Keyboard, Rtc, Eeprom) | **Kept**, same IDs, ports and commands. Eeprom loses its power-on copy (F9). The Monitor gets the native drawing commands and the extra modes (9.3). |
| **New** `mailbox.rs` | The CPU-side card exactly as built in Logisim: RX, TX, EVT latches, the six toggles, VEC, the synchroniser, STATUS, the decode and its mirrors (`0x00–0x0F`), and `0xFF` from empty ports. IN `0x00` and the vector step take their data from it. |
| **New** `iop.rs` (I/O processor model) | Takes TX bytes, runs the protocol against `Iod` + `Peripherals`, writes RX replies and moves events from `Iod`'s queue into EVT. The service time per byte is in CPU cycles (Q-C4), and so is the start-up delay before READY. |
| `board.rs`, `machine.rs` | IN / OUT go to the mailbox instead of the IOD's latches. The hardware interrupt's (port, data) comes from the EVT latches instead of `Iod::pop_interrupt`, with the same handshake (INT low in the vector step). **`cpu.rs` doesn't change.** |
| IDE | The monitor window, the keyboard (`keymap.rs`), the RTC sync and the EEPROM view stay. The I/O view gets a mailbox panel (flags, latches, the TX / RX / EVT byte log) next to the port and queue view, which now shows the I/O processor's latches. The EEPROM boot option goes. |
| Runner | Unchanged, apart from the machine config. |

**Shipped code** (the counts are IN / OUT instructions found in the repo):

| File | Change |
|---|---|
| `compiler/lib/asm/hbc2.has` (30), `screen.has` (46), `terminal.has` (11), `time.has` (7), `math/misc/random.has` (5) | Each `in` / `out` becomes a call to `mbx_in` / `mbx_out` (or a macro): about 100 sites, mechanical. `in()` / `out()` in C keep their signatures. `screen.has` and `terminal.has` can later move to PUTS / primitives for speed. |
| `compiler/runtime/devices.has`, `hbc2/templates/interruptHandler.has` | **Unchanged** (port-0 plug-in, IDs and per-port vectors kept). |
| `compiler/runtime/crt0.has`, `init.has` | Wait for READY before `main`, because the ESP32 starts after the CPU (Q-C7). The device wait already in `init.has` stays. |
| `compiler/runtime/boot.has` + the EEPROM build target (`hbc2-asm`, `hbc2-cc`, `hbc2-cli`, IDE) | **Removed**: no power-on copy (F9). |
| `hbc2/templates/main.has`, `main.c` | Unchanged, or one line on READY. |
| `compiler/tests` | Pass unchanged once the emulator routes through the mailbox. New tests: the native commands, a handler sending commands while `main` waits on a reply. |
| `logisim/tests/cpu/*.has`, the bench stub | No change (the CPU bench keeps its stub). |
| Docs | The Instruction Manual's IOD chapter and `Documentation/Peripherals/README.md`: the mailbox (ports 0x00 / 0x01, STATUS, events) and the protocol. The per-device pages stay. `HBC-2_Input_Output.pdf`: redrawn for the mailbox. ABI.md §8: READY and no EEPROM boot. |

Section 5b (clean break) is mostly moot: old programs keep working once rebuilt with the new library. Only programs using the EEPROM boot, or their own direct `in` / `out` instructions, need changes.

### 9.6 Open questions (with recommendations)

| # | Question | Recommendation |
|---|---|---|
| Q-C1 | I3 / I6 "off by 50 ns after the edge": the mailbox needs 96 / 63, and the CPU's own drivers already need 63–68. | Change I3 / I6 to match the CPU's own drivers (short overlap accepted, as for the ROMs' tDF in ir-mem 11.4), and measure the overlap on the breadboard (a B-check like B4). |
| Q-C2 | Two MCU → CPU channels (RX polled + EVT interrupt), or one? | **Two.** EVT keeps today's per-port interrupts and the data in I, so the handlers and `devices.has` stay. RX lets replies be read with interrupts off. One channel would save 2 × 595 and 2 flip-flops but would break every interrupt user. |
| Q-C3 | CTRL 595 (1 GPIO) or one GPIO per request (4 more GPIOs, −2 chips)? | CTRL 595 on the TTGO VGA32. Separate pulses on a bare DevKit. |
| Q-C4 | Emulated ESP32 speed. | A configurable service time, default 4 µs per byte (8 cycles), and READY after a configurable start-up delay (default 0 in the IDE, "realistic" 500 ms as an option). Measure the real figures in step C3 and set the defaults from them. |
| Q-C5 | VGA mode for the old monitor. | 640 × 480 × 16 colours (FabGL VGA16Controller), the 256 × 192 monitor drawn ×2 (512 × 384, centred). Its 42 × 24 text mode is drawn with the 6 × 8 font. A native 80 × 25 terminal as mode 1. |
| Q-C6 | Time source. | NTP once at start-up, then Wi-Fi off. A DS3231 on I²C later. The RTC device keeps its ports and the one-second interrupt either way. |
| Q-C7 | The ESP32 starts later than the CPU (CPU boot copy about 33 ms; ESP32 + FabGL a few hundred ms). | crt0 waits for READY. The MCU sets READY only once its plug-in events are queued, so the existing device wait then finds the devices. |
| Q-C8 | `rst_n` to the MCU. | Yes, as an input. The MCU resets its protocol state and sends the plug-in events again, as the emulator does at power-on. |
| Q-C9 | Second I/O processor (Arduino Nano). | Not now. `0x10–0x1F` stays decoded and free. |
| Q-C10 | Metastability of the asynchronous flags. | One `clk` stage (500 ns to settle) is enough at 2 MHz. Note it in iod-requirements, no second stage. |
| Q-C11 | 74AHCT595 in a 74HCT design. | Accept it as the rig already does (no DIP 74HCT595 from TI). Add its figures to chips.md in step C1. |
| Q-C12 | Native protocol details (sprites, sound, files). | Freeze only compat + the v1 table now. Add commands as the firmware grows, with INFO's version byte. |

### 9.7 Step order

1. **C1. Logisim `mbx` module** (conventions.md, real 74HCT chips, the 595 / 165 as their #TTL or chip models):
   - **What it does:** decode, latches, toggles, synchroniser, INT, the vector-step drivers.
   - **Tested against the real CPU** (cputest), with a test-bench model of the MCU side driving RCLK, SCK and MOSI:
     - IN / OUT on every port and mirror, and empty ports;
     - TX full (OUT ignored) and RX empty (IN ignored);
     - events: two in a row, an event arriving during a handler, an event while `main` waits on RX;
     - reset in the middle of a transfer;
     - contention checked.
   - **Wrap-up:** add the paths to `timing.py`, the final chip list and the behaviour the emulator must match to iod-requirements.md (I10+), Q-C1's change to I3 / I6, and the 595 figures to chips.md. **Stop for review.**
2. **C2. Emulator:** `mailbox.rs` exactly as C1, `iop.rs` with the compat protocol over the kept `Iod` + peripherals, routing in `board.rs` / `machine.rs`. The Logisim sequences of C1 become emulator tests, so both agree.
3. **C3. ESP32 firmware:**
   - **Start:** the protocol core in portable C++, shared with a host test build that replays C1's sequences.
   - **First on the rig:** the Arduino Mega rig (5 V, already 595 / 165 chains) stands in for the CPU and drives the mailbox card, to test the ESP32 side on real chips.
   - **Then:** VGA (FabGL), PS/2, SD, time and sound on the TTGO VGA32.
   - **Measure** the per-byte time and the start-up delay (Q-C4).
4. **C4. Library, templates and docs:** `mbx_in` / `mbx_out`, READY in crt0, removing the EEPROM boot target, then the native PUTS / primitives in `terminal.has` / `screen.has`. Docs as in 9.5.
