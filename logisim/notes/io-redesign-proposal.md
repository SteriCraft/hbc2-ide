# I/O redesign: replace the IOD with standard chips (proposal)

Status: **proposal for Gianni's review, 2026-11-10. Nothing is changed in the code.** The CPU does not change: IN / OUT, IE / RW, INT / INR and the vector table at `0xFE00` stay as they are (iod-requirements.md I1–I9).

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
