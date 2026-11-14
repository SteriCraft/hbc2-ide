# I/O mailbox (`iod`): build and test record

The C-min mailbox of iod-requirements.md ("Target design", I10–I16), built 2026-11-11 for step C2 of io-redesign-proposal.md 9.9. The design and its reasons are in the proposal (sections 9, 9.8, 9.9). This file records what was built, how it was tested, and what is still open.

## What was built

| File | What |
|---|---|
| `designs/iod.py` → `circuits/iod/iod.circ` | the module: 12 chips (U1–U12), wired by tunnels; `notes/bom-iod.csv` |
| `designs/74595.py` → `circuits/chips/74595.circ` | 74HCT595 / SN74AHCT595 model (Logisim has none), from sn74ahct595.pdf; `test_74595`: 2,610 rows pass |
| `designs/test_74165.py` → `circuits/chips/test_74165.circ` | Logisim's own 74165 against the CD74HCT165 truth table: 1,310 rows pass |
| `designs/test_iod_cpu.py` → `circuits/iod/test_iod_cpu.circ`, `test_iod_cpu_eager.circ` | the whole CPU + the card + a test MCU (a ROM state machine on `osc`) that follows the 1 µs rule, or breaks it (eager) |
| `tests/iod/mbx.has` | the CPU-level test program, self-checking (`; expect[bench]`) |
| `tools/cputest.py --bench iod / iod_eager` | runs the new benches. The IN data comes from the mailbox, so every IN step's byte is recorded and given to the emulator (`emutrace --in LINE:VALUE`). The program's expected registers are checked on the last line. |
| `tools/timing.py --section iod` | the card's paths. `bus_data`, `bus_addr` and `clk_5` now count the module's real loads instead of the estimates. |

**Ports.**
- **CPU side:** `iod_bus_in` (8), `iod_addr_in` (8, only A0, A1, A6 and A7 used), `iod_ie`, `iod_rw`, `iod_inr`, `clk`, `clk_wr_n`, `rst_n`; outputs `iod_bus_out`, `iod_addr_out`, `iod_int`.
- **ESP32 side:** `iod_sck`, `iod_mosi`, `iod_rclk_rx`, `iod_rclk_evt`, `iod_mbx_sel`; outputs `iod_miso`, `iod_mcu_tx_full`, `iod_mcu_rx_full`, `iod_mcu_evt_pend`, `iod_mcu_rst_n` (the LVC244's outputs).

## Tests

| Test | Result |
|---|---|
| `python designs/iod.py --test`: 4 sequences, now from the emulator (C3, below; C2 used a Python model, 13,816 rows) | **13,740 rows pass**. (1) every port 0–255, IN and OUT, IE on and off: who drives `bus_data`, what loads TX; (2) both handshakes, 40 rounds, MISO bit by bit, STATUS at each stage, and the 1 µs rule broken at random: an RX written during the clear step is lost; (3) 30 events: INT, the vector step (A0–A7 = p, `bus_data` = d, INT 0), EVT cleared in the next step, events written during the clear lost; (4) reset with every flag set: all clear, latches kept. Buses always checked (value or float). |
| Three wiring mutants of the module (rx_clr from /Y3, INT never cleared, A5 decoded instead of A6) | all caught (5,422 / 582 / 6,678 failing rows) |
| `cputest.py run tests/iod/mbx.has --bench iod` (the real CPU, the MCU keeps the rule) | **PASS**: no contention; the trace is identical to the emulator's (1,398 lines, interrupts placed by line and by cycle); 12 interrupts; x = 0 (no wrong byte, event data in I checked by the handler), y = 0C (12 events), c = 0 (no reply lost) |
| `cputest.py run tests/iod/mbx.has --bench iod_eager` (the MCU writes as soon as the flag reads 0) | **PASS** with the expected losses: x = 0, y = 08, c = 04. The 4 second replies and 4 second events were written during the CPU's clear and lost, exactly the race I15 rules out. |
| `cputest.py suite` (the old bench, after the `flag_reg` rename and the cputest changes) | 5 / 5 pass |
| `timing.py --section all --duty 45 / 50 / 55` | all limits met at 25 °C. iod: worst margin 4 ns (IN drivers off by 96 ns, limit 100, I3 as amended). At 85 °C (information) that path is 21 ns over. |

**The CPU-level program covers** (`tests/iod/mbx.has`):
- OUT TX on mirrors `0x20`, IN RX on `0x3C`, STATUS on `0x01` and `0x05`;
- OUTs to the empty ports `0x02` and `0x40`, which must not reach TX (the counts would be off);
- one and two replies per byte;
- one and two events per byte (two with interrupts off until the second event waits);
- the event data checked in the handler.

**Reset in the middle of a transfer, at CPU level: done 2026-11-14.** Three benches, `test_iod_cpu_reset_tx / _rx / _evt.circ` (`designs/test_iod_cpu.py`, `cputest.py --bench iod_reset_*`): the rule-keeping MCU pulls the CPU's reset once (its ROM's `rst_req` clears the reset RC, a `fired` register keeps it from coming back), and `cputest`'s observer starts again at the second boot; that run is compared with the emulator and checked against `expect[iod]`, and exactly one reset must be seen.

| Bench | Reset at | Result |
|---|---|---|
| `iod_reset_tx` | line 16: reading TX (MBX_SEL low, 4 bits shifted), first byte | **PASS**: 137,408 steps, second boot 65,537 copy writes, 1,398 lines identical to the emulator, 12 interrupts, x = 0, y = 0C, c = 0 |
| `iod_reset_rx` | line 292: phase 2, reply 1 in RX, reply 2 half shifted | **PASS**, the same |
| `iod_reset_evt` | line 1103: phase 4, event 1 pending (INT high, interrupts off), event 2 half shifted | **PASS**, the same |

In all three: no contention, every control line at its pulled level while `rst_rc` is high (C24, the mid-run reset included), and no stale reply or event after the reset (the program's counts are those of a clean run). Nothing to change on the card. One bench trap found on the way: `rst_req` must not depend on `m_rst_n`, or the reset it causes ends the pulse before `fired` is set and the reset repeats at every boot.

## Decisions taken while building

| # | Decision | Why |
|---|---|---|
| K1 | `main`'s test output `flags` is renamed `flag_reg` (cpu.circ: the output pin and the two tunnels; designs/cpu.py; cputest; both benches). No logic changes. | The layout commit 287c727 had lost the three `flags` labels: Logisim-evolution strips a label equal to the name of a circuit in the project (the `flags` module), as with `clk` (cu-decisions.md U10). The pin had been unconnected since then, and cputest reads the flags from it. `flag_reg` matches no circuit name. |
| K2 | A8–A15 pull-downs are **not modelled** in Logisim (they float in the vector step, as with the old stub). | A Pull Resistor inside `iod` would make its output pin drive 0 strongly onto `bus_addr` and fight the latch. A pull in `main` would put a component in `main`. The CPU never reads A8–A15 in the vector step. Board only: io-redesign-proposal.md 9.9 D3, breadboard check B5 (b). |
| K3 | ~~The module test's reference is the cycle-level model in designs/iod.py.~~ Done in C3: the reference is now the emulator (`hbc2 vectors iod`), and the Python model is gone. | conventions.md: the emulator's own code is the reference. |
| K4 | Logisim's 74LVC244 is a 74244 instance: `bom.py` counts it with the STATUS 74HCT244 (qty 2; chips.md lists both references in the row). | One Logisim chip, two orderable parts. |
| K5 | The test MCU reads MISO after MBX_SEL falls (QH shows bit 7 right after the load), then 7 SCK rises. It shifts 16 bits for every write: a reply sends V twice, so the RX 595 keeps the last 8. | One state machine for RX and EVT. |

## Open

- **85 °C (information):** the IN drivers turn off in up to 121 ns. The overlap with the next step's driver is measured by B5 (a).
- **The 165's data hold after SH/LD** isn't given by TI. The strobe rises by 410.5 ns and `bus_data` is held to 500, so about 90 ns of margin.
- **MBX_SEL rising while SCK is low** is a clock edge on the real 165 (CLK and CLK INH are ORed). It shifts a byte already read, which is harmless. The Logisim chip's behaviour there isn't checked.
- ~~**Reset at CPU level**~~ done 2026-11-14 (above).

## C3: the emulator (2026-11-11)

| File | What |
|---|---|
| `hbc2-core/src/mailbox.rs` | The card, chip by chip: one function per edge or level (`clk_rise`, `mbx_sel_rise`, `rclk_rx_rise`, `rclk_evt_rise`, `sck_rise`, `settle`), the outputs, and the 138 decode. On top: the CPU side, one step at a time (`cpu_in`, `cpu_out`, `clock`), and the MCU side (`mcu_write_rx`, `mcu_write_evt`, `mcu_read_tx`). |
| `hbc2-core/src/iop.rs` | The ESP32 firmware model. It polls once per CPU cycle, after the `clk` edge. Compat commands `0x01 p v` (OUT) and `0x02 p` (IN), plus `0x00` NOP and `0x03` INFO (protocol 1, firmware 0 = emulator, devices bitmap; replaced on 2026-11-11 by the descriptors of D6, see "Device list by INFO" below); other bytes are ignored. Every queued `Iod` interrupt becomes an event, in order. It keeps I15 (`rule_cycles`, default 2 = 1 µs) and takes `service_cycles` per action (default 8 = 4 µs, Q-C4). |
| `board.rs`, `machine.rs` | `MachineConfig::mailbox: Option<IopConfig>`. **None (the default) is the old IOD**, unchanged: the shipped library still uses `in` / `out` directly until C5. With Some, IN / OUT go to the card, INT and the vector step's (port, data) come from it, and the card and the IOP run cycle by cycle, also during an instruction's wait cycles (skipped while nothing can change). |
| `cpu.rs` | Two lines: IN and OUT call `board.io_read` / `board.io_write` instead of the IOD directly. With no mailbox they do exactly what they did. (9.5 said "cpu.rs doesn't change": the CPU's behaviour and the hardware don't.) |
| `hbc2-cli/src/vectors/iod.rs` | `hbc2 vectors iod`: C2's 4 sequences, driving `mailbox.rs`'s chip functions from pin rows (edges found as Logisim does). `designs/iod.py --test` now uses them; the Python `Card` model is removed. |

**Tests**

| Test | Result |
|---|---|
| `designs/iod.py --test` (Logisim module against `hbc2 vectors iod`) | **13,740 rows pass** |
| Two mutants of `mailbox.rs`: EVT never cleared after the vector step; STATUS bit 2 from INT instead of EVT pending | both caught. The first is caught by the vector generator's own checks and by the machine tests. **The second is caught only by Logisim** (10 rows), so the program-level tests alone aren't enough. |
| `hbc2-core` unit tests (mailbox.rs): the I10 decode, RX lost during the clear, the TX round trip, the event handshake | pass |
| `hbc2-asm/tests/mailbox.rs`: the same program with `in` / `out` on the old IOD and with the compat protocol through the mailbox (monitor text mode, 20 WRITEs, READ, RTC GET_TIME and IN, a free port's latch, two key events) | **identical**: the event log (port, I) of 3 plug-ins + 2 keys, the RAM results, the screen, the registers. INFO = 1, 0, 0x07. With the rule broken (service 1, rule 0), the MCU writes all 6 plug-in events and the CPU's `evt_clr` loses some; with the rule they all arrive. A halted CPU with a quiet mailbox is idle, and a key press wakes it. |
| Workspace tests, old IOD (`mailbox: None`) | all pass, unchanged |
| `cputest.py suite`, `run tests/iod/mbx.has --bench iod / iod_eager` (emutrace rebuilt on the new core) | suite 5 / 5 pass; iod PASS (12 interrupts, 1,398 lines identical to the emulator); iod_eager PASS (8 interrupts, the expected losses) |
| Speed (a CPU-bound loop, release build) | about 240 MHz emulated with the mailbox, 390 without (the hardware runs at 2 MHz) |

**Open after C3**
- **Q-C4 defaults** (8 cycles per action, 2 for the rule) are guesses until the firmware is measured in C4.
- **Native commands** (PUTS, PIXEL, GET_TIME, SD…, 9.3) aren't in the model yet. They'll be added together with the firmware, so both get the same set; INFO's version byte tells them apart.
- **Not switched on yet:** the IDE, the runner and the compiler tests still use the old IOD (`mailbox: None`). Turning the mailbox on needs the library changes (C5). The IDE's mailbox panel comes then too.
- **`rst_n` without a power cycle:** the machine has only power-on, which resets the card and the IOP together.

## C5: the library on the mailbox (2026-11-11)

| File | What |
|---|---|
| `compiler/lib/asm/mailbox.has` (new, first in `lib.has`) | `mbx_out` / `mbx_in`: one compat command each (`0x01 p v`, `0x02 p` + the reply), sent with interrupts off (`psf` / `cli` / `pof`, so the I flag comes back as it was). They keep every register and every flag. Calls: `out P, V` → `psh P, V` / `cal mbx_out` / `asp 2`; `in R, P` → `psh P` / `cal mbx_in` / `pop R` (the reply replaces the pushed port). Defines `MBX_DATA`, `MBX_STATUS`, the STATUS bits and the command bytes. |
| `hbc2.has` (30), `screen.has` (46), `terminal.has` (11), `time.has` (7), `math/misc/random.has` (5) | Every `in` / `out` replaced as above, by a script: 99 sites, no other change. `in()` / `out()` in C keep their signatures. |
| `compiler/runtime/boot.has` | Its own copies, `boot_out` / `boot_in`: the library's `mbx_*` sit past `$0x0200` and aren't copied yet when it runs. Their `boot_` names put them under hbc2-cc's 512-byte check. The EEPROM target still works in the emulator (see open points). |
| `MachineConfig::default()`, the IDE (`emulator.rs`) | `mailbox: Some(IopConfig::default())`. So the IDE, `hbc2 run` / `bench` / `trace --peripherals`, the runner and the compiler's program tests all use the mailbox. Tests of the CPU itself (`bare()` configs, emutrace, the helper calls in hbc2-cc's helpers.rs) keep the old IOD. |
| `hbc2-asm/tests/assemble.rs` | The Assembly Language manual's first example still writes the monitor with raw `out`: its test runs on the old IOD (`mailbox: None`) until the manual changes. |
| `hbc2-ide/tests/smoke.rs` | The smoke test's program sends its three OUTs as `0x01 p v` commands. |

**Tests:** the whole workspace, 331 tests, pass. Among them, all compiler test programs run through the mailbox: the 5 `.screen` comparisons (text and pixels through `mbx_out`), the `.keys` inputs (keyboard events through EVT), `eeprom.c` built for the EEPROM target (boot.has's copy loop through the mailbox), `time.c`, `random.c`, `multi/vector`.

**Cost:** one `mbx_out` takes about **210 CPU cycles** (measured: 100 calls in a loop, about 105 µs each at 2 MHz), against 4 for a raw `out`. Most of it is CPU instructions (the STATUS polling, `psf` / `psh` / `cal`), not the I/O processor's 3 × 8 cycles. A monitor text cell (5 OUTs) is now about 0.5 ms, a full 42 × 24 screen clear about 2.5 s. The native commands (PUTS, CLEAR, RECT…, 9.3) are the fix, with C4.

**Docs to change.** Gianni, later on 2026-11-11: Claude edits them all except the `.odt` (the Instruction Manual), which Gianni edits. **Done** for every row below but the Instruction Manual; the Assembly Language manual's example now sends `0x01 p v` itself and its test is back on the default config; the EEPROM loader example includes `mailbox.has` (checked: it assembles in the first 512 bytes and copies `main` through the mailbox). `HBC-2_Input_Output_diagram.pdf` is redrawn (source `Documentation/src/HBC-2_Input_Output_diagram.html`; Gianni added `rst_n`).

| Doc | Where | What changes |
|---|---|---|
| `Documentation/HBC-2 Assembly Language.md` | opening example, lines 7-31 | Raw `out` to the monitor's ports 1-5 no longer reaches the monitor. Rewrite with `mbx_out` (and `.include` of `mailbox.has`, or a few lines sending `0x01 p v`). Then put `language_documentation_example_prints_hi` back on the default config. |
| | line 78, the `in out` row (94) | Still true for the CPU, but the ports are the mailbox's now: `0x00` data, `0x01` STATUS (mirrors in `0x00-0x3F`). Device ports are reached with `mbx_out` / `mbx_in`. |
| | "EEPROM target" (427-440), lines 303 and 456 | Depends on the EEPROM decision below. |
| `Documentation/Peripherals/README.md` | "The I/O Driver (IOD)", lines 16-60 | The IOD (latches, port numbers, plug-in, queue) now lives in the I/O processor; the CPU reaches it through the mailbox. The `out d, a` / `in a, d` example becomes the `mbx_out` / `mbx_in` calls. "The device runs the command right away, before the next instruction" becomes: a few µs later, and before the next command is read, so an IN after an OUT sees its result. |
| | Interrupts (from line 53) | The 256-entry queue is in the I/O processor, delivered one at a time through EVT. Same vector, I = data, same handlers. |
| `Peripherals/Monitor/Reference.md` (15 `in` / `out`), `RTC/Reference.md` (13), `EEPROM/Reference.md` (12) | the examples | Raw `in` / `out` → the `mbx_*` calls. Ports, offsets and commands don't change. EEPROM: the power-on copy, per the decision below. Keyboard: no change. |
| `Documentation/src/HBC-2_IDE_Instruction_Manual.odt` → `HBC-2_Instruction_Manual.pdf` | IOD chapter | "256 ports, port 0 reserved for CPU <> IOD", IOD, queue → the mailbox card (ports, STATUS, TX / RX / EVT, the vector step with A8-A15 pulled down) and the ESP32 behind it. The instruction pages (IN, OUT, INT / INR) don't change. |
| `compiler/ABI.md` | §8 line 257; §8 EEPROM target (262+) | "already waiting in the IOD queue" → in the I/O processor's queue. EEPROM: per the decision. |
| `compiler/lib/README.md` | file list (32+) | Add `asm/mailbox.has`; `screen.has` / `terminal.has` / `hbc2.has` do their I/O through it. |
| `hbc2/README.md` | line 12 (`hbc2-core`), line 97 | hbc2-core also has the mailbox card and the I/O processor model. "Nothing runs on every cycle except the CPU and IOD" → the mailbox card and the I/O processor also run each cycle, skipped while idle. |

**Open after C5 (library part)**
- **EEPROM target:** ~~open~~ **removed** (Gianni, 2026-11-11: the program lives in the boot ROM, and building fills RAM exactly as burning the two AT28C256 would). Gone: `Target` in hbc2-asm (images are 64 KiB, addresses up to `$0xFFFF`), `boot.has`, the EEPROM image of hbc2-cc, `--eeprom` on `hbc2 asm` / `cc`, the IDE's target setting. The EEPROM stays as **storage**: it copies nothing at power-on, `MachineConfig` plugs a blank one by default, the IDE has an EEPROM checkbox and keeps its content from one machine to the next (Load / Save EEPROM content), `hbc2 run --eeprom file.bin` loads it. `eeprom.c` tests storage only; `EEPROM_FREE` is gone.
- **Later (Gianni, 2026-11-11): load programs over USB.** A small loader in the boot ROM asks the ESP32 for the program through the mailbox and writes it into RAM; the IDE sends the program to the ESP32 over USB. Avoids pulling and burning the two AT28C256 for every test. Not designed yet.
- **Assembly projects:** ~~open~~ **done.** A new assembly project gets `mailbox.has` (`hbc2/templates/mailbox.has`, a copy of the library's, checked by a test), included by `main.has`.
- **IDE mailbox panel** (9.5): ~~open~~ **done.** The I/O view has the mailbox card (STATUS, the flags, TX / RX / EVT) and the I/O processor (command so far, replies waiting, byte and event counts, and a log of its last 100 commands and events with the device of each port: `Iop::log`). The snapshot carries the `Iop`.
- **Screen speed** (above): native commands with C4.
- **Start-up on the hardware:** ~~open~~ **done** (D6, "Device list by INFO" below). Was: crt0 waits a fixed 32 loop turns for the plug-in interrupts (init.has). In the emulator the I/O processor is ready at power-on and all of them are handled during the `.bss` clear (checked with the wait cut to 1 turn). On the board the ESP32 starts after the CPU, so `main` would start with every port at 0. D5 assumed init.has waits for the events; it doesn't. Proposal: crt0 sends INFO (`mbx_in` blocks until the ESP32 answers) and waits until every device in INFO's bitmap has its port.

## Device list by INFO (D6, D7: Gianni, 2026-11-11)

The CPU asks which devices exist; the ESP32 announces nothing. The ESP32 doesn't hold the CPU's reset. No mouse and no audio (the mailbox takes 21 of 22 GPIOs, one spare; the GPIO 36 question is gone). io-redesign-proposal.md 9.10.

| Part | What |
|---|---|
| `hbc2-core/src/iop.rs` | INFO replies: protocol version (1), N, N descriptors of 6 bytes (class 2 bytes, product ID 2 bytes, first port, number of ports), from `Peripherals::descriptors` in port order. `IopConfig::boot_cycles` (0 by default): the firmware reads nothing until then, so a command waits in TX. `Iop::rst_n` drops a half-received command and the unread replies; ports, queue and devices are kept. |
| `iod.rs`, `peripherals/` | `Iod::connect` sends no interrupt; owners are product IDs. Each peripheral has `CLASS` and `PRODUCT` instead of `DEVICE_ID`. |
| `machine.rs` | `Machine::reset`: the reset button (CPU, mailbox card, boot ROM copy; the I/O processor gets `rst_n` only). In the IDE since 2026-11-11 (see below). |
| `compiler/lib/asm/devices.has` | `dev_init`: INFO, each byte waited for 2 s at most (24-bit loop, 38 cycles a turn, 105,263 turns = 4,000,000 cycles; `DEV_TIMEOUT_*`), the table (`dev_table`, 16 entries, `dev_count`, `dev_protocol` = 0 without an I/O processor), unset vectors of device ports -> `dev_ignore`. `dev_find` (class, subclass or `DEV_ANY`, n-th), `dev_port`. Copied to `hbc2/templates/devices.has` (test). |
| crt0 | `cli`, .bss clear, `dev_init`, `crt0_devices` (the four port variables; the EEPROM by product ID), `sti`, `main`. The 32-turn wait and the port-0 handler are gone. |
| `hbc2.h` | `struct device`, `io_protocol`, `device_count`, `device_get`, `device_find`, `DEVICE_*` classes, `PRODUCT_*`. |
| Template | `main.has` calls `dev_init` and sets `monitorPort`, `keyboardPort`, `rtcPort`, `eepromPort`; `interruptHandler.has` is gone. The New project option is "Find the devices at startup (devices.has)". |

**Tests:** ESP32 ready at once (startup under 20,000 cycles), a 1 s boot, absent (timeout at 4.0 M cycles, empty table), a CPU reset after it (answers at once), `dev_find` by class, `compiler/tests/devices.c`, the reset dropping a half-received command. Every compiler test program passes. The empty C program grew from 269 to 604 bytes.

**Open**
- The firmware (C4) must do the same: INFO's descriptors, `rst_n` dropping a partial command.
- The emulator's ESP32 boots in 0 cycles by default (`boot_cycles`); 2,000,000 would be the board's.
- ~~The IDE has no reset button yet~~ Done 2026-11-11, Gianni's decision: the IDE's **Reset** button (Ctrl+F5) now calls `Machine::reset`; a new **Restart** button to its right (Ctrl+Shift+F5) does the power cycle (`Machine::power_on`). The runner has `Runner::reset` next to `power_cycle`.

## The TTGO VGA32 wired to the card (Gianni, 2026-11-14)

Gianni drew these changes in `kicad/iod/hbc2_iod.kicad_sch` on 2026-11-14. KiCad's ERC reports 0 errors and 0 warnings, and `kicadcmp.py` reports no wiring error.

**The board.** The LilyGO TTGO VGA32 v1.4 stays. It sits off the I/O card, in a 3D-printed box next to it, and is wired to the card by three cables. It isn't socketed on the card, and no DevKitC is used. The pins below were checked against its schematic: `vga32_v1.4.pdf`, in the LilyGO/FabGL repository on GitHub, folder `Schematic`.

**What the TTGO schematic shows:**
- **SD card:** GPIO 2 / 12 / 13 / 14. Its pull-ups (R34–R37) are not fitted.
- **Header J2 (2 × 4, 2.54 mm):** pin 1 IO12, 2 IO14, 3 IO13, 4 IO2, 5 IO34, 6 IO39, 7 GND, 8 +3V3.
- **PS/2 mouse port:** carries IO26 (CLK) and IO27 (DAT) through MOSFET level shifters, with 4.7 kΩ pull-ups to 5 V on the connector side and 22 kΩ to 3.3 V on the GPIO side. Its pin 4 is the board's +5V.
- **IO25:** the audio output, through 270 Ω, then 150 Ω to GND, then a 10 µF capacitor to the jack. The jack is DC-blocked.
- **IO35:** the battery divider (100 kΩ / 100 kΩ).
- **IO36:** button K1 (10 kΩ pull-up).
- **IO0:** the auto-program circuit.

**Mailbox pins.** These replace the table in io-redesign-proposal.md 9.4 and 9.8.

| Signal | GPIO | Card connector | TTGO end | Why |
|---|---|---|---|---|
| MOSI | 12 | J5.1 | header J2.1 | shared with the SD card |
| SCK | 14 | J5.2 | header J2.2 | shared with the SD card |
| (SD CS) | 13 | J5.3, not connected | header J2.3 | the TTGO's own SD card |
| MISO | 2 | J5.4 (U12 1Y0 through 33 Ω) | header J2.4 | shared with the SD card; the LVC244 drives it only while MBX_SEL is low |
| tx_full | 34 | J5.5 (U12 2Y3 through 33 Ω) | header J2.5 | input-only pin |
| rx_full | 39 | J5.6 (U12 2Y2 through 33 Ω) | header J2.6 | input-only pin |
| GND, +3V3 | — | J5.7, J5.8 | header J2.7, J2.8 | the TTGO's 3.3 V supplies U12 |
| RCLK_EVT | 27 | J6.1 | mouse port pin 1 (DAT) | **idle high**: the TTGO pulls it up |
| GND | — | J6.3 | mouse port pin 3 | |
| +5V_ESP | — | J6.4 (from D3) | mouse port pin 4 (VCC) | powers the TTGO |
| MBX_SEL | 26 | J6.5 | mouse port pin 5 (CLK) | idle high from reset (the TTGO's pull-up) |
| RCLK_RX | 25 | J7.1 | wire soldered at R28's ESP32 end | idle low: the audio RC (420 Ω to GND) is its pull-down |
| evt_pend | 35 | J7.2 (U12 2Y1 through 33 Ω) | wire soldered at the battery divider's midpoint | the LVC244 overrides 50 kΩ |
| rst_n | 36 | J7.3 (U12 2Y0 through **220 Ω**) | wire soldered at K1 (S_VP side) | pressing K1 can't short the LVC244 |
| GND | — | J7.4 | any GND | return for the soldered wires |

The other pins stay as FabGL's defaults for this board. VGA uses 21, 22, 18, 19, 4, 5, 23 and 15, and the PS/2 keyboard uses 33 (CLK) and 32 (DAT). That makes 21 GPIOs in use, with GPIO0 spare. Nothing goes on GPIO0: it's held low while the board is being flashed, so it can't carry MBX_SEL (that would enable MISO onto GPIO2) or an RCLK.

**Why these pins:**
- **MBX_SEL must be high from reset.** On GPIO25 it would be pulled low while the ESP32 boots, and its rising edge when the firmware takes the pin would clear TX. That would lose the INFO command crt0 sends at start-up.
- **The mouse port's pull-ups keep MBX_SEL and RCLK_EVT high** through an ESP32 reset. So the RCLK_EVT store happens on the rising edge at the end of a low pulse; the 595's RCLK and the 74's CLK only see rising edges, so this is the same edge as before.
- **RCLK_RX is idle low**, like the Logisim test MCU, which needs no change.
- **H3's pull resistors are not needed on the card**, because the TTGO's own resistors set every idle level.

**Card changes (KiCad):**
- **U12 → SN74LVC244AN on +3V3**, which comes from the TTGO through J5.8. C12 decouples it on +3V3. It is the only 3.3 V chip in the HBC-2. There is no regulator on the card: U12 is then powered exactly when the ESP32 is, so it can't back-power the ESP32's pins.
- **R1–R5:** 33 Ω in series on U12's outputs; 220 Ω on `rst_n` (R2).
- **D3, a 1N5817:** from +5V to `+5V_ESP` (J6.4). Together with the TTGO's own BAT20J on USB VBUS, it makes a diode OR. The card never feeds the PC's USB, and USB never feeds the HBC-2, so the TTGO can be flashed while the HBC-2 runs. The TTGO's ME6211 regulator still gives 3.3 V from about 4.3 V.
- **J1:** the CPU's signals only: `iod_ie`, `iod_rw`, `iod_inr`, `clk`, `clk_wr_n`, `rst_n`, `iod_int`. `iod_int` goes to the control unit (`cu_int`, cu J1.6).
- **J5:** a 2 × 4 header numbered like the TTGO's J2, for a straight IDC ribbon. Use a keyed (shrouded) header.
- **J6:** a 1 × 5 header numbered like the mini-DIN; pin 2 not connected.
- **J7:** a 1 × 4 header for the soldered wires.
- **RN1:** a 10 kΩ × 8 bussed network (Bourns 4609X-101-103LF, symbol `Device:R_Network08`, footprint `Resistor_THT:R_Array_SIP9`). Its common pin is on GND, and its 8 resistors pull `iod_addr8..15` down. A8–A15 reach J4, now 1 × 16.
- **U3, U6, U9:** relabelled SN74AHCT595N.

**Firmware rules:**
- RCLK_EVT idles high and is pulsed low. RCLK_RX idles low and is pulsed high. MBX_SEL idles high.
- MBX_SEL and the SD card's CS (GPIO13) are never low at the same time.
- Set a low drive strength on SCK (`gpio_set_drive_capability`), so the ribbon doesn't ring.
- No speaker on the TTGO: every RCLK_RX pulse would click through its audio amplifier.

**Logisim:** no change. The card ends at the ESP32 interface, and the test MCU's idle-low RCLK_EVT gives the same rising edge.

**Breadboard check:** B7 in `breadboard-checks.md`.
