# IOD: what the CPU side requires

The IOD (I/O device: port latches, interrupt queue, peripherals) is out of scope for now. This file is the contract the CPU's sections were built against, so a future IOD design starts from it. Source: ir-mem-decisions.md section 4 ("CPU-side IOD interface") and Gianni's answer 7 (2026-11-05). Times are from the rising `clk` edge that starts the step, at max 25 °C (the pass criterion); the CPU's step is 500 ns.

| ID | From | Requirement | Why |
|---|---|---|---|
| I1 | ir / mem (2026-11-05) | **Connector lines**: `bus_data` (8, bidirectional), `bus_addr` (16; the CPU drives it except in the vector step), RW and IE (CPU → IOD), INT (IOD → CPU), INR (CPU → IOD), `clk`, `clk_wr_n`, `rst_n`. | The manual's lines, plus the clocks the synchronous contract needs. |
| I2 | Gianni's answer 7 (2026-11-05) | **Synchronous to `clk`, no WAIT line**: the IOD answers every transaction within the step, at fixed times (I3–I6). | The manual's handshake is "one clock cycle"; the control unit (C21, C22) never waits. |
| I3 | ir / mem | **IN** (IE = 1, RW = 1, one step): `bus_addr` = 00:port is valid by 72 ns; the IOD drives `bus_data` by **322 ns** (335 with an exact 50 % oscillator; 322 with the 45–55 % duty, cu-decisions.md Q9), holds it to the edge, and is off **within 100 ns** after it (amended 2026-11-11, Q-C1: was 50 ns; the CPU's own `bus_data` drivers take 63–68 ns, so a short overlap with the next step's driver is accepted as for C5, and measured on the breadboard, B5). | The register write's setup (control-requirements.md C10). |
| I4 | ir / mem | **OUT** (IE = 1, RW = 0, one step): `bus_addr` = 00:port by 72 ns, `bus_data` = R2 by 242 ns, both held to the edge; the IOD captures `bus_data` on the **rising edge of `clk_wr_n`** (about 375 ns). | Data valid more than 100 ns before it, held 125 ns after. |
| I5 | ir / mem; Gianni (2026-11-05) | **INT comes from a flip-flop clocked by `clk`** and is **valid by 150 ns** after the rising edge (was 250, provisional); high while the queue isn't empty. The CPU samples it only at instruction boundaries (and in the halt loop). | The microcode's boundary decision goes INT → 2 gates → the condition multiplexer → EEPROM → pipeline register and needs INT by 219 ns at 25 °C (cu-decisions.md section 6, Q6). Synchronous: no synchronizer on the CPU side. |
| I6 | ir / mem (C12, C21) | **Hardware interrupt**: the IOD sees INR = 1 at the edge ending step n; **in step n + 1 only** (the vector step) it holds INT = 0, drives **A0–A7** of `bus_addr` = p by **237 ns** and `bus_data` = data by **335 ns**, and is off within 100 ns after the edge ending it (Q-C1, as I3). **A8–A15 are not driven by anyone** in the vector step (the CPU's latch is off): a 10 kΩ pull-down network on the I/O card holds them at 0 (amended 2026-11-11, Gianni: was "drives 00:p"). INT may rise again from step n + 2. INT may also stay high for the first tens of ns of the vector step: the CPU never samples it there (I5: boundaries and the halt loop only). | The address section computes FE00 + 2p from A0–A7 only (addr-decisions.md A8, the `a_ivt` path); nothing reads A8–A15 in the vector step (RAM not enabled, ROM /OE off since M8). |
| I7 | Gianni's answer 7 (2026-11-05) | **A microcontroller-based IOD** (a future redesign) can't answer IN or the vector step within 335 ns from firmware. It must answer through **hardware latches and a FIFO** that the CPU reads directly at the times above (e.g. a 74HCT574 per input port, a hardware FIFO of port + data for the interrupt queue, a 74HCT574 or 377 capturing OUT on `clk_wr_n`), which the microcontroller **fills and empties asynchronously**. INT then comes from the FIFO's "not empty" flag, resynchronised to `clk` (I5). | Keeps the CPU's contract fixed whatever the IOD's implementation. |
| I8 | ir / mem | **Bus discipline**: the IOD drives `bus_data` only in IN and vector steps, and `bus_addr` only in vector steps; never two drivers on a bus (contention fails the tests). | control-requirements.md C5, C11. |
| I9 | datasheet re-check (2026-11-10, ir-mem-decisions.md section 11) | **Levels (recommended)**: rail-to-rail outputs (74HCT or CMOS) on `bus_addr` and `bus_data`; inputs TTL-compatible (VIH 2.0 V) are fine, the CPU drives with HCT. | `bus_addr` reaches the AS6C1008's address pins directly, and the RAM needs VIH 0.7 × VCC = 3.5 V; the IOD only drives `bus_addr` in the vector step, when the RAM isn't accessed, so TTL-level outputs would still work. |

The test benches use a simplified IOD stub (ir-mem-decisions.md M5): its queue and port latches are in the test vector generator, and its bus drivers are tri-state buffers in the bench, checked for contention like any other driver. The whole-CPU trace bench (circuits/cpu/test_cpu.circ, cu-decisions.md section 15, U16) has its own stub: port latches in a 256-byte RAM and one interrupt source armed by the program (OUT to a port F0–FF).

## Target design for the `iod` module: the C-min mailbox (Gianni, 2026-11-11)

Decided 2026-11-11 (io-redesign-proposal.md sections 9 and 9.8): the IOD is a **mailbox of latches and flip-flops** in front of an ESP32 I/O processor, which runs today's IOD model (port latches, peripherals, plug-in interrupts, event queue) behind a byte protocol. The VIA / 16C550 target of 2026-11-10 is withdrawn (its timing fails, section 8). The CPU and the microcode don't change.

### Chips (12, one card)

| Chip | Qty | Job |
|---|---|---|
| 74HCT138 | 1 | decode: G1 = IE, /G2A = A7, /G2B = A6, A = A0, B = RW, C = A1 |
| 74HCT32 | 1 | TX strobe `tx_wr_n` = OR(/Y0, `clk_wr_n`) (3 gates spare) |
| 74HCT74 | 4 | flags TX, RX, EVT; `rx_clr`, `evt_clr`, VEC, `evt_s` (INT) (1 spare) |
| SN74AHCT595N | 3 | RX data → `bus_data`; EVT port → `bus_addr` A0–A7; EVT data → `bus_data` |
| CD74HCT165E | 1 | TX data, loaded from `bus_data`, shifted out to the MCU |
| 74HCT244 | 1 | STATUS → `bus_data` |
| SN74LVC244AN (3.3 V) | 1 | 5 V → 3.3 V for the ESP32: MISO (enabled by MBX_SEL), `tx_full`, `rx_full`, `evt_pend`, `rst_n` |
| 10 kΩ bussed network | (passive) | pulls `bus_addr` A8–A15 to 0 (I6) |

### Behaviour (what the emulator must match)

| ID | Rule |
|---|---|
| I10 | **Ports.** Only `0x00–0x3F` with A1 = 0 answer, A2–A5 ignored (mirrors): IN port ≡ 0 (mod 4) = **RX**, IN port ≡ 1 = **STATUS**, OUT port ≡ 0 = **TX**, OUT port ≡ 1 = nothing. Ports ≡ 2, 3 (mod 4) in `0x00–0x3F` are a free second select (`/Y4–/Y7`), and `0x40–0xFF` are empty. IN from an empty port leaves `bus_data` undriven (floating on the board, `0xFF` in the emulator, Q11). |
| I11 | **STATUS** = bit 0 RX_AVAIL (RX full), bit 1 TX_FREE (TX empty), bit 2 EVT_PEND, bits 3–7 = 0. Read live (not synchronised): from the CPU's side a flag only changes towards "go ahead" (TX_FREE only falls when the CPU writes, RX_AVAIL only when it reads), so a read that catches a change gets either value and both are right. |
| I12 | **TX** (CPU → MCU): OUT TX loads the 165 while `clk_wr_n` is low and sets TX full (74 /PRE). The MCU reads it (MBX_SEL low: 165 clock enabled, MISO driven; 8 SCK rising edges, MSB first; QH shows bit 7 before the first edge) and clears TX full on MBX_SEL's rising edge. An OUT TX while TX is full overwrites the byte (no protection; the library checks TX_FREE first). |
| I13 | **RX** (MCU → CPU): the MCU shifts 8 bits into the RX 595 (MOSI, SCK rising, MSB first) and pulses RCLK_RX: the byte appears and RX full is set on the same rising edge. IN RX drives the byte; RX full clears during the **step after** the IN RX (`rx_clr`, a 74 on `clk` with D = /Y2). |
| I14 | **EVT** (MCU → CPU, interrupt): the MCU shifts 16 bits (the data byte, then the port byte) and pulses RCLK_EVT: port and data latch, EVT pending is set. `evt_s` samples it on `clk` and is **INT** (I5). In the vector step (VEC = INR delayed one step) the EVT port 595 drives A0–A7 = p and the EVT data 595 drives `bus_data` = d (→ I); `evt_s` is cleared (/CLR = /VEC), and EVT pending clears in the **step after** the vector step (`evt_clr`, D = /VEC). |
| I15 | **Firmware rule** (the one race left): the MCU writes RX (or EVT) only when RX full (EVT pending) reads 0, and **at least 1 µs after it last went to 0**: the CPU's clear holds the flag's /CLR low for one 500 ns step, and a set during it is lost. TX has no race (each side acts only on the other state of the flag). |
| I16 | **Reset** (`rst_n` low): TX, RX, EVT, VEC and INT clear (RX and EVT through `rx_clr` / `evt_clr`, INT one `clk` edge later); the 595 / 165 contents are kept. The MCU sees `rst_n` and resets its protocol state. |

The CPU-level bench (circuits/iod/test_iod_cpu.circ) replaces the stub's port latches and interrupt source with the real module and a test MCU; the older trace bench (circuits/cpu/test_cpu.circ) keeps its stub.
