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
| `python designs/iod.py --test`: 4 sequences from the cycle-level model in iod.py (`Card`) | **13,816 rows pass**. (1) every port 0–255, IN and OUT, IE on and off: who drives `bus_data`, what loads TX; (2) both handshakes, 40 rounds, MISO bit by bit, STATUS at each stage, and the 1 µs rule broken at random: an RX written during the clear step is lost; (3) 30 events: INT, the vector step (A0–A7 = p, `bus_data` = d, INT 0), EVT cleared in the next step, events written during the clear lost; (4) reset with every flag set: all clear, latches kept. Buses always checked (value or float). |
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

**Not covered at CPU level: reset in the middle of a transfer.** It is covered at module level (sequence 4). `cputest`'s observer follows one boot: a reset in the middle of a run restarts the 65,548-step boot copy, which the trace and the emulator comparison don't model. It could be added (the test MCU pulling `rc`, and an observer that restarts on a second reset) if wanted.

## Decisions taken while building

| # | Decision | Why |
|---|---|---|
| K1 | `main`'s test output `flags` is renamed `flag_reg` (cpu.circ: the output pin and the two tunnels; designs/cpu.py; cputest; both benches). No logic changes. | The layout commit 287c727 had lost the three `flags` labels: Logisim-evolution strips a label equal to the name of a circuit in the project (the `flags` module), as with `clk` (cu-decisions.md U10). The pin had been unconnected since then, and cputest reads the flags from it. `flag_reg` matches no circuit name. |
| K2 | A8–A15 pull-downs are **not modelled** in Logisim (they float in the vector step, as with the old stub). | A Pull Resistor inside `iod` would make its output pin drive 0 strongly onto `bus_addr` and fight the latch. A pull in `main` would put a component in `main`. The CPU never reads A8–A15 in the vector step. Board only: io-redesign-proposal.md 9.9 D3, breadboard check B5 (b). |
| K3 | The module test's reference is the cycle-level model in designs/iod.py, not the emulator. | The emulator gets its mailbox in step C3. Then `hbc2 vectors iod` replaces the model (conventions.md), and these sequences become emulator tests. |
| K4 | Logisim's 74LVC244 is a 74244 instance: `bom.py` counts it with the STATUS 74HCT244 (qty 2; chips.md lists both references in the row). | One Logisim chip, two orderable parts. |
| K5 | The test MCU reads MISO after MBX_SEL falls (QH shows bit 7 right after the load), then 7 SCK rises. It shifts 16 bits for every write: a reply sends V twice, so the RX 595 keeps the last 8. | One state machine for RX and EVT. |

## Open

- **85 °C (information):** the IN drivers turn off in up to 121 ns. The overlap with the next step's driver is measured by B5 (a).
- **The 165's data hold after SH/LD** isn't given by TI. The strobe rises by 410.5 ns and `bus_data` is held to 500, so about 90 ns of margin.
- **MBX_SEL rising while SCK is low** is a clock edge on the real 165 (CLK and CLK INH are ORed). It shifts a byte already read, which is harmless. The Logisim chip's behaviour there isn't checked.
- **Reset at CPU level** (above).
