# Instruction register and memory / bus interface: decisions

**Phase 1 (paper design, 2026-11-05)**, then **phase 2 (built and tested the same night)**: Gianni's answers and the phase 2 decisions M1–M7 are in section 9, the result in [ir-mem-report.md](ir-mem-report.md). Sections 2–5 are the phase 1 design; where M1 (RAM WE# from `clk_mem_wr_n`, not `clk_wr_n`) and M4 (a third 74HCT541 for the ROMs, 8 chips in `mem`) changed it, section 9 wins. Section 11 re-checks the memories against the current datasheets (2026-11-10).

Source of truth: `hbc2-core/src/isa.rs` (instruction format, `DECODE`, `Instruction::fetched`), `cpu.rs`, `board.rs`, `machine.rs`, `peripherals/eeprom.rs`, and the manual (`Documentation/HBC-2_Instruction_Manual.pdf`: "Instruction format", "RAM and I/O interfaces", "Hardware interrupts"). Requirements collected from control-requirements.md (C5, C8, C9, C10, C12–C17), registers-requirements.md and pc-sp-requirements.md (P1). Out of scope: the control unit (what it must do is in section 5), the real IOD and the peripherals.

Planned files: module `ir` in `circuits/ir/ir.circ` (designs/ir.py), module `mem` in `circuits/mem/mem.circ` (designs/mem.py), chip models `circuits/chips/as6c1008.circ` and `at28c256.circ`, the IOD stub in the test benches only.

## 1. What the emulator does

### 1.1 Instruction format (isa.rs, checked against the manual)

Bytes in memory order (big-endian `u32`, `Instruction`):

| Byte | Bits 7..0 | Fields |
|---|---|---|
| 0 | `o5 o4 o3 o2 o1 o0 m3 m2` | opcode (6), mode bits 3..2 |
| 1 | `m1 m0 a2 a1 a0 b2 b1 b0` | mode bits 1..0, R1 (bits 5..3), R2 (bits 2..0) |
| 2 | V1 | V1; R3 = V1 bits 2..0 (`Instruction::r3`, mode 4 only) |
| 3 | V2 | V2; Vx = V1:V2 (V1 high) |

So the 4-bit mode **straddles bytes 0 and 1**. Length (`DECODE`, 64 × 16 table):

| Case | Bytes read | Then |
|---|---|---|
| opcode ≥ 0x36 (invalid) | 1 | NOP, whatever the mode bits say ("skips no further bytes") |
| valid opcode without operands (mode None only: NOP, CLx, STx, HLT, IRT, RET, PSF, POF) | 1 | runs if m3 m2 = 00; **stops** (H set, I cleared, PC stays) if not. m1 m0 are never read |
| valid opcode with operands, mode listed for it | `AddrMode::length`: Reg 2, Reg16 2, RegImm8 3, StackOffset 3, Imm8 3, RegRam 4, mode 4 4, Imm16 4 | runs |
| valid opcode with operands, mode not listed (or 9–F) | 2 | **stops** |

`Instruction::fetched` clears the bytes past the length. The hardware doesn't: those IR bytes keep the previous instruction's values. No instruction uses a field outside its length (checked against `execute`: R2 only in Reg / Reg16 / mode 4, V1 only from RegImm8 up, V2 only in 4-byte modes), so the stale bytes are harmless, but **for a 1-byte instruction m1 m0 are stale** and the decode must not look at them (section 5, C18).

### 1.2 Who reads each IR field

| Field | Read by | When |
|---|---|---|
| opcode, mode | control unit (decode, sequencing) | the whole instruction, from the step after their byte is loaded |
| R1, R2, R3 + inverted bit 2 of each | register section (`regs_r1/r2/r3`, `regs_r*_2_n`, C8) | from the step after byte 1 (R1, R2) / V1 (R3) is loaded |
| V1, V2 | address section OFFSET (`addr_v1`, `addr_v2`): Vx, SP + V1, R1:R2 + V2, 00:V1 (INT), SP ± V1 / Vx | from the step after they are loaded |
| V1 as an ALU operand (RegImm8), MOV R1, imm | **not from the IR**: the register section captures V1 from the data bus while it is fetched (operand latch, registers-decisions.md D2; C9) | the fetch step of V1 |

So the IR drives no bus; it is a plain register with its outputs wired to the control unit, the register section and the address section.

### 1.3 IR loads other than the fetch (C14)

| Event | Byte read | Into |
|---|---|---|
| RET, IRT | return address high, low (pop) | V1, V2, then PC := 0 + Vx |
| hardware / software interrupt entry | IVT entry high, low (FE00 + 2p) | V1, V2, then PC := 0 + Vx |

V1 and V2 therefore need their own load enables, usable in any step. R3 (V1 bits 2..0) is overwritten too, which is harmless: none of these events uses R3.

### 1.4 Every memory and I/O access (cpu.rs)

| Access | R / W | Address (via the address latch, P1) | Data | Lines (manual) |
|---|---|---|---|---|
| fetch, 1–4 bytes | R | PC … PC + 3 | → IR (and the operand latches, for V1) | RE, RW = read |
| ALU / CMP / LOD operand: RegRam, mode 4, StackOffset | R | Vx, R1:R2 + V2, SP + V1 | → operand latch / register | RE, read |
| STR RegRam / mode 4 / StackOffset | W | same | R1 or R3 → bus_data | RE, write |
| INC / DEC / NOT on memory (Imm16, mode 4) | R, then W | Vx, R1:R2 + V2 | read → operand latch A; ALU result → bus_data | RE |
| PSH, PSF; CAL (PC lo, PC hi); interrupt entry (I, PC lo, PC hi) | W | SP − 1 (each) | register / flags / PC byte → bus_data | RE, write |
| POP, POF; RET (2), IRT (3) | R | SP, SP + 1, SP + 2 | → register / flags / V1, V2 / I | RE, read |
| interrupt vector | R | FE00 + 2p, + 1 | → V1, V2 | RE, read |
| IN R1, R2 | R | 00:R2 | IOD → bus_data → R1 | IE, read |
| OUT R1, R2 | W | 00:R1 | R2 → bus_data → IOD | IE, write |
| INT (software, Imm8) | — | 00:V1 on bus_addr, I on bus_data (emulator `board.address_bus / data_bus`) | captured by the pending port latch (C13); no RE, no IE | none |
| hardware interrupt, vector step | — | the IOD drives 00:p | the IOD drives the data → I | none (the IOD drives) |
| boot (power-on, `Machine::power_on`) | W | every address that holds the image | the image | section 3 |

The memory map is flat: **64 KiB of RAM at 0000–FFFF** (`RAM_SIZE`), the stack below FE00 (`STACK_RESET`, first push at FDFF), the IVT at FE00–FFFF (`IVT_START`, 2 bytes per port, MSB first), the program at 0000 (`PROGRAM_START`). Nothing is memory-mapped: I/O is a separate space (IE), the IVT is ordinary RAM that programs write. So the hardware needs no address decoding for RAM.

### 1.5 Boot in the emulator

`Machine::power_on`: with an EEPROM plugged in, RAM is cleared, then the EEPROM peripheral copies **0000–01FF and FE00–FFFF** from its own memory into RAM (`BOOT_RANGES`) before the CPU runs; the rest of RAM is 0. Without an EEPROM, a 64 KiB RAM image is copied as is. The EEPROM's specification (`Documentation/Peripherals/EEPROM/Specifications.txt`) says the original idea was the EEPROM **holding the CPU through a HALT pin while it dumps the data**, i.e. DMA by a peripheral. The IDE's reset is a power cycle (`worker.rs` calls `power_on`), so the RAM is reloaded on every reset.

### 1.6 Hardware interrupt handshake (manual, board.rs `tick_iod`, cpu.rs)

1. IOD: INT high while its queue isn't empty.
2. CPU: at an instruction boundary, with I = 1 and no STI delay, raises INR (for one clock, manual). In the hardware the pushes come first (C12, addr Q2).
3. IOD: on INR, INT low, and for **one clock** the port on the address bus and the data on the data bus. In the emulator this happens in the same tick as INR; the CPU reads them on the next tick.
4. CPU: reads the port and the data (→ I), I := 0, INR low, jumps through the IVT.
5. IOD: next clock, INT high again if the queue isn't empty.

Software interrupts don't use the handshake (C13).

## 2. Instruction register (`ir`)

### Block diagram

```
bus_data (ir_bus_in) ──┬──────────────────────────────┬────────────┬────────────┐
                       │ 377 byte 0 (/E ir_b0_ld_n)   │ 377 byte 1 │ 377 V1     │ 377 V2
                       │  → ir_op (6), ir_mode 3..2   │ (ir_b1_ld_n)│ (ir_v1_ld_n)│ (ir_v2_ld_n)
                       │                              │ → ir_mode 1..0, ir_r1, ir_r2
                       │                              │            │ → ir_v1, ir_r3 = V1 2..0
                       │                              │            │            → ir_v2
   bit 5 ─→ 14 (NOT) ─→ 377 X1 (/E ir_b1_ld_n): ir_r1_2_n
   bit 2 ─→ 14 (NOT) ─┬→ 377 X1:                ir_r2_2_n
                      └→ 377 X2 (/E ir_v1_ld_n): ir_r3_2_n
```

- **6 × 74HCT377** (edge-triggered on `clk`, data enable /E, no gated clock, the same part as the address section) + **1 × 74HCT14** (2 inverters used). Byte 1 holds 10 bits (mode 1..0, R1, R2, not R1 bit 2, not R2 bit 2) and V1 9 (V1, not R3 bit 2), and the extra bits must load with their own byte, so X1 (with byte 1) and X2 (with V1) are separate 377s, mostly empty (Q3 for the alternatives).
- The inverted bits are inverted **before** the latch (C8: an inverter after it would add about 28 ns to the register read path). One inverter (NOT bus bit 2) feeds both X1 (R2's bit 2, in the byte 1 step) and X2 (R3's bit 2, in the V1 step).
- No reset: the IR is undefined until the first fetch, and nothing reads it before (the emulator's `ir = 0` at reset has no visible effect).
- **No decode in the IR** (Q4): the control unit gets the raw opcode and mode; the 64 × 16 decode (length, stops, invalid opcode = NOP) belongs in its microcode ROM, which is indexed by them anyway. A gate-level "opcode valid" would be 2 gates (`op ≥ 0x36` = o5·o4·(o3 + o2·o1)), but the mode check is a per-opcode table, so it needs the ROM regardless.

### Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `clk` | 1 | top | every 377 samples /E on the rising edge |
| `ir_b0_ld_n`, `ir_b1_ld_n`, `ir_v1_ld_n`, `ir_v2_ld_n` | 1 each | top | 0: load that byte from the data bus on the next edge (any step: C14) |
| `ir_bus_in` | 8 | left | `bus_data` |
| `ir_op` | 6 | right | opcode (byte 0 bits 7..2) |
| `ir_mode` | 4 | right | mode: bits 3..2 from byte 0, 1..0 from byte 1 |
| `ir_r1`, `ir_r2`, `ir_r3` | 3 each | right | → `regs_r1`, `regs_r2`, `regs_r3` |
| `ir_r1_2_n`, `ir_r2_2_n`, `ir_r3_2_n` | 1 each | right | → `regs_r*_2_n` (C8) |
| `ir_v1`, `ir_v2` | 8 each | right | → `addr_v1`, `addr_v2` (V1:V2 = Vx) |

### Chips: 7

| Qty | Part | Role |
|---|---|---|
| 6 | 74HCT377 | byte 0, byte 1, V1, V2, X1 (not R1 / R2 bit 2), X2 (not R3 bit 2) |
| 1 | 74HCT14 | NOT bus bit 5, NOT bus bit 2 (4 gates spare) |

## 3. Boot

The program must be in RAM before the first fetch at 0000, and the RAM's power-up content is random. Options:

| Option | Chips | Matches the emulator | Trade-offs |
|---|---|---|---|
| **A (recommended). Boot ROM copied by the control unit.** 2 × AT28C256 (64 KiB) on the memory section's local data bus. After the reset, a boot loop in the control unit: each step latch, PC := PC + 1 while the ROM drives the local bus and the RAM writes it at the latch's address (RAM[a] := ROM[a]), 65,536 steps (**33 ms**); the step that copies FFFF wraps PC to 0000, and the adder's carry out ends the loop, so the CPU starts at 0000 with the latch already there. | 2 ROMs + 1 inverter (A15 picks the ROM); the control unit's loop; **one new pin on `addr`** (the carry out, Q2) | Yes, exactly: the ROM image is the emulator's power-on RAM (the EEPROM build's two ranges and zeros elsewhere, or a 64 KiB RAM image). Every byte of RAM is defined at start (the emulator's zeros included). | Reuses PC, the adder and the latch as the copy counter: no counter chips. Also on every reset (the IDE's reset is a power cycle). The EEPROM peripheral loses its power-on DMA and becomes a port-only peripheral (Gianni's redesign); the assembler needs a `--bootrom` output (two 32 KiB halves). The ROM's data never reaches `bus_data`. |
| B. DMA by a board-level copier (the EEPROM spec's HALT pin) | counter (4 × 161), 2 × 541 address drivers, ROM, a few gates: about 8 + ROM | Yes (same image) | The CPU must float its buses on request (the reset word already does, C16), but RE / RW / /WE need a second master. More chips; it is what the original EEPROM spec described. |
| C. ROM mapped over RAM at 0000–01FF and FE00–FFFF, switched off by a port write | ROM + decoder + a flag flip-flop + gates: about 4 | **No**: the program must switch the ROM off; until then the IVT can't be written; reads of those ranges differ | Saves the copy time; changes the programming model (the emulator and the assembler would have to follow). |
| D. Copy only the two ranges (1 KiB) | A + a 2-gate address test | Only for EEPROM builds | Needs the control unit to test address bits 9 and 15; a RAM-image build couldn't boot. 33 ms is not worth it. |

**Recommendation: A.** No counter, no second bus master, the same RAM content as the emulator in both build modes. The ROMs: **2 × AT28C256-15PU** (Microchip, 32 KiB parallel EEPROM, PDIP-28, 150 ns; active at Arrow, findchips life cycle active, reprogrammable with a TL866-class programmer). A single 64 / 128 KiB part would save the inverter and one ROM, but the usual one, **SST39SF010A in PDIP-32, is end of life** (Microchip PCN, last order 2026-07-01); its PLCC-32 version is still active (a socket, neither DIP nor SOIC).

The boot loop, step by step (control unit, C20):

| Step | Address section | Memory |
|---|---|---|
| reset (C16) + register sweep (C7) | PC = 0000, SP = FE00 | idle |
| 0 | latch := PC (0000) | idle |
| 1 … 65,536 | latch, PC := PC + 1 | ROM drives the local bus (`mem_rom_oe_n` = 0), RAM writes it (`mem_we_n` = 0, the step's `clk_wr_n` pulse); buffers off |
| the step with latch = FFFF | the sum wraps to 0000, `addr_cout` = 1 → the control unit leaves the loop | copies FFFF |
| next | the first fetch (latch = PC = 0000 already) | read |

## 4. Memory and bus interface (`mem`)

### Block diagram

```
                     bus_addr (16, in: mem_addr) ─────────────┬───────────────┬───────────────┐
                                                              │ A0–A15        │ A0–A14        │ A0–A14
   clk_wr_n ─┐                                         AS6C1008-55        AT28C256 (lo)   AT28C256 (hi)
  mem_we_n ──┴─ 32 (OR) ──→ /WE                         A16 = 0           /CE = A15       /CE = NOT A15 (14)
  mem_re ───────────────→ CE2   (CE# = GND)                               /OE = mem_rom_oe_n (both), /WE = VCC
  mem_oe_n ─────────────→ /OE ─┐                              │               │               │
                               │       mem_d (local data bus, 8) ─────────────┴───────────────┘
                               │          │                              ↑
                               └→ 541 read buffer: mem_d → bus_data (mem_bus_out, /OE = mem_oe_n)
                                  541 write buffer: bus_data (mem_bus_in) → mem_d (/OE = mem_wbuf_oe_n)
  mem_rw, mem_ie (manual RW, IE): to the IOD connector only
```

- **RAM: AS6C1008-55PCN** (Gianni's choice: 128 K × 8, A16 tied low, half unused). Alliance Memory, PDIP-32 600 mil; DigiKey lists it **active**, Octopart "production" (data about 6 months old). PCN is 0–70 °C; the industrial **-55PIN** (−40–85 °C) exists if the 85 °C column should mean something for the RAM too. Datasheet used: AS6C1008 v1.0 (February 2007, a distributor copy); phase 2 takes the current Alliance revision into `circuits/chips/`.
- **A local data bus `mem_d`** for the RAM and the ROMs, joined to `bus_data` by **two 74HCT541** (one per direction):
  - the SRAM's outputs are only rated at 2 mA (VOL 0.4 V at IOL 2 mA, test load 30 pF), and `bus_data` is about 380 pF: on paper the RAM can't drive it in time (the 6 mA 541 is what timing.py knows how to derate);
  - the boot copy (ROM → RAM) stays off `bus_data`;
  - two 541s instead of one 74HCT245: a Logisim module pin is one-way, so the 245's B side can't be both `mem_bus_in` and `mem_bus_out` (the same split as `regs_bus_in` / `regs_bus_out`). One more chip, but a part whose delays are already in chips.md.
- **The control unit decodes the strobes** (C19): `mem_oe_n` drives both the RAM's /OE and the read buffer (they are always on together), `mem_we_n` gates `clk_wr_n` into /WE, `mem_wbuf_oe_n` the write buffer, `mem_rom_oe_n` the ROMs. `mem_re` drives CE2 (the manual's RE). The manual's RW and IE go to the IOD connector; the memory doesn't need them. One OR gate is the only logic on the strobe (the same single gate as the 670 writes, so the same write window).
- **Write = `clk_wr_n` OR `mem_we_n`** (/WE-controlled write, CE held all step): /WE is low from about 250 to 375 ns, so the address (from the latch, changed only at the edge) is stable around the whole pulse. This is the same race as the 670 (C6, registers-decisions.md D7), solved the same way.
- **I/O**: nothing in `mem`. RE and IE are never both high (C19): an IN or OUT step leaves the RAM and both buffers off, and the IOD uses `bus_data` and `bus_addr` directly. P1 already gives 00 on the high byte.

### Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `clk_wr_n` | 1 | top | the write phase (C6) |
| `mem_re` | 1 | top | RE: RAM enabled (CE2) |
| `mem_oe_n` | 1 | top | 0: RAM read: RAM /OE and the read buffer onto `bus_data` |
| `mem_we_n` | 1 | top | 0: RAM write during `clk_wr_n` |
| `mem_wbuf_oe_n` | 1 | top | 0: `bus_data` → local bus (CPU write) |
| `mem_rom_oe_n` | 1 | top | 0: the boot ROM drives the local bus (boot copy) |
| `mem_addr` | 16 | left | `bus_addr` (read only) |
| `mem_bus_in` | 8 | left | `bus_data`, in |
| `mem_bus_out` | 8 | right | `bus_data`, out, tri-state (read buffer) |
| `mem_d` | 8 | right | the local bus, as an output for the tests (contention check) |

Allowed combinations (anything else is a control unit error):

| Step | `mem_re` | `mem_oe_n` | `mem_we_n` | `mem_wbuf_oe_n` | `mem_rom_oe_n` | Drives `bus_data` |
|---|---|---|---|---|---|---|
| idle, I/O, vector step | 0 | 1 | 1 | 1 | 1 | — |
| RAM read | 1 | 0 | 1 | 1 | 1 | mem |
| RAM write | 1 | 1 | 0 | 0 | 1 | the CPU source |
| boot copy | 1 | 1 | 0 | 1 | 0 | — |

### CPU-side IOD interface (the IOD itself is out of scope)

Lines on the connector: `bus_data` (8, bidirectional), `bus_addr` (16; the CPU drives it except in the vector step), **RW, IE** (CPU → IOD), **INT** (IOD → CPU), **INR** (CPU → IOD), and `clk`, `clk_wr_n`, `rst_n`. Proposed contract (Q7), times from the rising `clk` edge that starts the step, max 25 °C:

| Transaction | CPU | IOD |
|---|---|---|
| IN (IE = 1, RW = 1, one step) | `bus_addr` = 00:port by 71 ns | drives `bus_data` by **335 ns**, held to the edge, off by 50 ns after it |
| OUT (IE = 1, RW = 0, one step) | `bus_addr` = 00:port by 71 ns, `bus_data` = R2 by 209 ns, both held to the edge | captures `bus_data` on the **rising edge of `clk_wr_n`** (about 375 ns: data valid more than 100 ns before it, and held 125 ns after) |
| INT | samples INT only at an instruction boundary | changes INT only just after a rising `clk` edge (it is synchronous to `clk`), valid by 250 ns (provisional, for the control unit's next-word logic) |
| hardware interrupt | after the three pushes, **INR = 1 for exactly one step** (step n) | sees INR at the edge ending step n, then **for step n + 1 only** (the vector step): INT = 0, `bus_addr` = 00:p by **237 ns** (C12), `bus_data` = data by **335 ns**; off by 50 ns after the edge ending it. INT may rise again from step n + 2 (manual step 5) |

The vector step is therefore always the step after INR: no waiting, no WAIT line (the manual's protocol is a fixed "one clock"). This makes C12's "wait for INT low" a fixed step (C21).

**IOD stub (test benches only, non-TTL parts allowed):** clocked by `clk`; a small FIFO (4 entries of port + data), filled from test-vector pins (`stub_raise`, `stub_port`, `stub_data`), INT = not empty; on INR at an edge it pops one entry and drives `bus_addr` = 00:port and `bus_data` = data for the next step only; port latches for IN / OUT (a 256 × 8 RAM: IN drives `bus_data` when IE and RW, OUT writes on `clk_wr_n`'s rising edge). Its outputs go through the bench's bus-contention checks like any other driver.

### Chips: 7

| Qty | Part | Role |
|---|---|---|
| 1 | AS6C1008-55PCN | 128 K × 8 SRAM, A16 low |
| 2 | AT28C256-15PU | boot ROM, 0000–7FFF and 8000–FFFF |
| 2 | 74HCT541 | read buffer (mem_d → bus_data), write buffer (bus_data → mem_d) |
| 1 | 74HCT32 | /WE = clk_wr_n OR mem_we_n (3 gates spare) |
| 1 | 74HCT14 | NOT A15 for the second ROM (5 gates spare) |

The RAM and ROM become chip models in `circuits/chips/` (`as6c1008`, `at28c256`): a Logisim #Memory RAM / ROM inside, the datasheet's pins outside (CE#, CE2, OE#, WE#), tested against the datasheet's truth table, labelled `U<n>` and counted by bom.py like the 74574 model (conventions.md: "RAM and EEPROM … use the real chip's pinout").

## 5. Timing estimate

Times in ns from the `clk` edge, **max 25 °C (pass) / max 85 °C (information)**, chips.md delays plus bus loading. Figures not yet in chips.md (from the datasheets, to be entered in phase 2): AS6C1008-55 tAA 55, tACE 55, tOE 30, tWP 45, tDW 25, tAS 0, tWR 0, tDH 0, tOHZ 20; AT28C256-15 tACC 150, tCE 150, tOE 70; **74HCT14 estimated at 32 / 40** (to replace by the SN74HCT14 figures). The SRAM and ROM figures are their full-range maximums (one figure for both columns).

| Path | Through | 25 °C | 85 °C | Limit | Margin 25 / 85 |
|---|---|---|---|---|---|
| **RAM read → register write** (the worst memory read) | clk → latch 377 (38 / 48) → 541 + bus_addr load (33 / 42) = 71 / 90 → RAM tAA (55) → read buffer 541 A → Y (23 / 29) + bus_data load (33 / 42) | **182** | **216** | 335 / 325 (C10) | **153 / 109** |
| RAM read, enables | control register (33 / 41) → read buffer OE → Y (30 / 38) + load (33 / 42) | 96 | 121 | 335 / 325 | 239 / 204 |
| RAM read, CE2 / /OE | control (33 / 41) → tACE (55) / tOE (30) → 541 (23 / 29) + load (33 / 42) | 144 | 167 | 335 / 325 | 191 / 158 |
| fetch into the IR (incl. the inverted bits) | RAM read (182 / 216) → 14 (32 / 40) → 377 setup (12 / 15) | 226 | 271 | 500 | 274 / 229 |
| RAM write: data | register → bus_data (209 / 266, timing.py r_dbus) → write buffer 541 (23 / 29), local bus light | 232 | 295 | earliest /WE rise − tDW = 347 − 25 / 340 − 25 = 322 / 315 | **90 / 20** |
| RAM write: address before /WE falls | clk → bus_addr (71 / 90), tAS 0 | 71 | 90 | earliest /WE fall 222 / 215 | 151 / 125 |
| RAM write: /WE pulse | the 670's analysis (one OR after `clk_wr_n`): 125 − FF − NAND − OR | **49** | **30** | tWP 45 | **4 / −15** |
| RAM write: hold | /WE rises by 428 / 440 (tWR = tDH = 0) | 428 | 440 | 500 | 72 / 60 |
| boot copy: ROM → RAM | clk → bus_addr (71 / 90) → 14 on A15 (32 / 40) → ROM tCE (150), local bus light | 253 | 280 | 322 / 315 | 69 / 35 |
| IR → register index (new: the IR is a 377, not the 574 the register section assumed) | 377 CP → Q (38 / 48) → 153 I → Y | +5 / +7 on the register read paths: end to end 412 → **417** / 520 → **527** | | 500 | 83 / −27 (info, as before) |
| IR → V1 / V2 → adder → PC (a_ir) | same +5 / +7 | 324 | 406 | 500 | 176 / 94 |

Every path passes at 25 °C. **The RAM write pulse is the tight one**: 4 ns of margin at 25 °C, 15 ns short at 85 °C on paper (with no minimum delays, as for the 670). The fallback already listed in chips.md (74ACT74 + 74ACT10 in the clock generator) gives about 75 ns at 85 °C (Q5).

**Bus loading** (worst bit; the timing.py estimates replaced in phase 2 by counts on the circuits):
- `bus_data`: the IR adds 5 inputs on bit 2 (4 × 377 + the 14), the estimate had 4; the memory adds 1 input + 1 output (the two 541s), the IOD 1 + 1; the estimate's "RAM, ROM, I/O: 3 outputs" goes away. **About 386 pF instead of 372** (+1 / +2 ns on a bus driver).
- `bus_addr`: RAM 1, 2 ROMs, the IOD 1 + 1 output, the pending port latch 1 + 1, `addr`'s IVT 1, the latch's driver: about 6 inputs, 3 outputs, **158 pF instead of 146** (+1 ns). A15 also drives the 14.
- `mem_d` (local): RAM, 2 ROMs, 2 buffers, about 60 pF: no derating needed for the 541 (rated at 50 pF); the RAM's and ROM's outputs see about twice their 30 / 100 pF test loads at most.

Levels: the AS6C1008 v1.0 datasheet gives VIH = 0.7 × VCC (3.5 V at 5 V). HCT outputs are rail-to-rail, so every RAM input driven by a 541, a 377 or the 32 is fine; the **ROM's outputs during the boot copy** must reach 3.5 V: check the AT28C256's CMOS VOH in phase 2 (and the current AS6C1008 revision, which may give 2.2 V).

## 6. Questions for Gianni (with my recommendation)

1. **Boot: option A** (the control unit copies a 64 KiB boot ROM, 2 × AT28C256, into RAM after every reset; 33 ms; the ROM image = the emulator's power-on RAM). The EEPROM peripheral's power-on DMA goes away (port-only), and the assembler gets a boot ROM output. **Recommended.** Alternatives: B (DMA copier, about 8 chips more), C (shadow ROM, changes the programming model).
2. **One new output on `addr`: `addr_cout`**, the carry out of the last 74HCT283 (its C4 pin is unconnected today), so the boot loop knows when PC wraps. No chip, but a port change on an accepted module (`tools/usages.py addr`: no user yet). **Recommended** over decoding PC = FFFF (a 16-input gate).
3. **The IR's three inverted bits: 6 × 74HCT377** (two of them hold 2 and 1 bits). Alternative with the same chip count: 4 × 377 + 2 × 74HCT173 (4-bit register with enables, smaller, but a new part and Logisim's 74173 unchecked). **Recommended: 377 only.**
4. **No decode logic in the IR**: the control unit's microcode ROM, indexed by opcode and mode, holds isa.rs's `DECODE` (length, stops, invalid = NOP). **Recommended**; the IR would otherwise need a ROM of its own.
5. **RAM write pulse: 49 ns vs tWP 45 ns at 25 °C (margin 4), −15 at 85 °C** (on paper, no minimum delays). Accept, with the 74ACT74 + 74ACT10 fallback and a breadboard check (B2: measure the /WE pulse at the RAM pin, pass ≥ 45 ns)? **Recommended.** Alternative: a dedicated, wider RAM strobe from the clock generator (a third phase, 1–2 chips).
6. **Local memory bus behind 2 × 74HCT541** (the SRAM is only rated 2 mA into a 380 pF bus; the boot copy stays off `bus_data`). **Recommended.** Alternative: the RAM straight on `bus_data` (−2 chips), which works in practice but can't be shown on paper.
7. **IOD contract: synchronous to `clk`**, fixed timing (the vector step is always the step after INR; the IOD answers by 237 / 335 ns; OUT data captured on the rising edge of `clk_wr_n`), no WAIT line, `clk` and `clk_wr_n` on the connector. **Recommended**, as the manual describes ("for one clock cycle"). If your IOD redesign is a microcontroller, it can't answer IN in 335 ns; then we'd need a WAIT input (the control unit holds the step), or hardware port latches in the IOD that the CPU reads directly.
8. **RAM grade**: AS6C1008-55**PCN** (0–70 °C, the one you named) or **-55PIN** (−40–85 °C, same price range)? **Recommended: PCN** (indoors; the 85 °C column is information only).
9. **Names**: modules `ir` (`circuits/ir/ir.circ`) and `mem` (`circuits/mem/mem.circ`; conventions.md listed `mem_if` as an example, but the module holds the memory itself). Chip models `as6c1008` and `at28c256` in `circuits/chips/`. **Recommended.**

## 7. For the control unit (to copy into control-requirements.md in phase 2)

- **C18 — IR loads and decode.** Drive `ir_b0_ld_n`, `ir_b1_ld_n`, `ir_v1_ld_n`, `ir_v2_ld_n` from the control register. Fetch byte 0; byte 1 only if the opcode takes operands; V1 and V2 as the mode's length says (isa.rs `DECODE`: invalid opcode = 1-byte NOP whatever the mode bits; no-operand opcode: stops if byte 0's mode bits ≠ 00; listed mode: `AddrMode::length`; otherwise stops after 2 bytes, then C15). Bytes past the length are not loaded and keep old values: never use a field past the length; for a 1-byte instruction **ignore `ir_mode` bits 1..0** (stale). A microcode ROM indexed by `ir_op` and `ir_mode` meets this if its contents for a 1-byte opcode don't depend on mode bits 1..0.
- **C19 — Memory strobes.** Drive `mem_re`, `mem_oe_n`, `mem_we_n`, `mem_wbuf_oe_n`, `mem_rom_oe_n` and the bus lines RW, IE from the control register, only in the combinations of section 4's table. RE and IE never both 1. A RAM read counts as a `bus_data` driver (C5's list: at most one). A RAM write's data source (register, result latch, PC / SP, flags) drives `bus_data` for the whole step.
- **C20 — Boot loop** (section 3): after the reset (C16, C17) and the register sweep (C7): one step latch := PC, then latch, PC := PC + 1 with the boot copy strobes, until `addr_cout` = 1 in a copy step; then the first fetch. No interrupt is taken before the first fetch.
- **C21 — Hardware interrupt timing** (refines C12): after the last push, INR = 1 for exactly one step; the next step is the vector step (the IOD drives `bus_addr` and `bus_data` then); no waiting. INT is sampled only at instruction boundaries.
- **C22 — I/O steps**: IN and OUT are one step with IE = 1 and the latch showing 00:port (P1): IN with RW = 1 (the IOD drives `bus_data`; R1 written in that step, C10), OUT with RW = 0 (R2 onto `bus_data`, `regs_a_bus_oe_n`). RAM strobes off.

## 8. Phase 2 plan (after the go-ahead)

1. Chip models `as6c1008` and `at28c256` (+ tests vs the datasheets; datasheets into `circuits/chips/`); 74HCT14, AS6C1008, AT28C256 delays into chips.md; parts tables.
2. `ir`: tests `hbc2 vectors ir` (model: random bytes and enables, fields checked against `Instruction`'s accessors, the inverted bits) and `ir_cpu` (the real CPU: every instruction fetched as the control unit will, by `size()` / `stops()`, plus the RET / IRT / vector reloads; fields checked where the emulator defines them).
3. `addr_cout` on `addr` (if Q2 is accepted), its tests re-run.
4. `mem`: tests `mem` (model: reads, writes, boot copy steps, random data, every output checked as a bus, `mem_d` included) and a combined bench (`addr` + `ir` + `mem` + the IOD stub, driven like `addr_cpu`): the boot copy against `Machine::power_on`'s RAM, fetches from real RAM, IN / OUT and queued hardware interrupts through the stub. Bus contention fails every test.
5. `timing.py --section mem` (and `ir`), the new loads; control-requirements.md C18–C22; README, chips.md, a report with what needs Gianni's review.

## 9. Gianni's answers (2026-11-05) — phase 2 go-ahead

Phase 2 runs unattended: take the recommended option on any new question, log it here (M1, M2, …), commit after each module, don't push, finish with notes/ir-mem-report.md and its "Need Gianni's review" list.

1. Boot: option A accepted (copy the 64 KiB ROM after every reset).
2. `addr_cout`, 3. 6 × 377, 6. local bus behind 2 × 541, 8. AS6C1008-55PCN, 9. names `ir`, `mem`, `as6c1008`, `at28c256`: accepted.
4. Accepted. **Decision: the control unit is microcoded** (EEPROMs indexed by opcode, mode and step, with a pipeline register on their outputs). To record in control-requirements.md.
5. **Not accepted as is**: widen the RAM write pulse by design first (the address is stable from 71 ns, the RAM needs tWP 45 ns), aiming for **≥ 30 ns margin at 25 °C** (pulse ≥ 75 ns). Keep B2 in breadboard-checks.md.
7. IOD synchronous to `clk`, no WAIT line: accepted. Record in the IOD requirements that a future microcontroller-based IOD must answer through hardware latches / a FIFO that it fills asynchronously.

### Phase 2 decisions

**M1 — RAM write strobe `clk_mem_wr_n` = /Q2 of the clock generator** (answer 5). FF2 (C6) already exists: it is `clk` sampled on the osc fall, so Q2 is high from about 125 ns to 375 ns of each step, and its /Q2 output is unused. The RAM's /WE becomes `clk_mem_wr_n` OR `mem_we_n` (the same 74HCT32 gate; no new chip), low about 125–375 ns instead of 250–375. Both edges come from the same flip-flop on osc falls 250 ns apart, so the pulse no longer depends on FF1 or the NAND3. Figures (max 25 °C / 85 °C, minimum delays 0, t = 0 = clk rising, which can be FF1's 28 / 35 ns after its osc edge):

| Check | Computation | 25 °C | 85 °C | Need | Margin 25 / 85 |
|---|---|---|---|---|---|
| pulse, shortest | 250 − FF2 (28 / 35) − NAND inverter (24 / 30) − OR (24 / 30) | 174 | 155 | tWP 45 | **129 / 110** (was 4 / −15) |
| falls, earliest | 125 − FF1 (28 / 35) | 97 | 90 | address valid 71 / 90 (tAS 0) | 26 / 0 |
| rises, earliest | 375 − FF1 | 347 | 340 | data valid + tDW 25: 232 / 295 | 90 / 20 |
| rises, latest (+ tWR = tDH = 0) | 375 + NAND + FF2 + OR | 451 | 470 | before the next edge (500): the address and data change only after it | 49 / 30 |

The new tight point is the address setup before /WE falls (26 ns at 25 °C, 0 at 85 °C, on paper with FF1 at max and FF2 at 0: the two are in the same 74HCT74 package, so in practice their delays track). Writing garbage into the cell during the pulse before the data arrives is harmless: a /WE-controlled SRAM write takes the data present tDW before the rise. `clk_mem_wr_n` is a new global signal (control unit's clock generator, C6 updated in phase 2); the 670 writes keep `clk_wr_n`. B2 (breadboard-checks.md) measures the /WE pulse and the address-to-/WE-fall setup at the RAM pins.

**M2 — the RAM model's data pins.** A Logisim subcircuit pin is one-way (SubcircuitFactory.propagate: input pins read, output pins drive), so a chip model can't have a bidirectional DQ pin. The `as6c1008` model has DQ0–DQ7 as tri-state outputs (pins 13–15, 17–21, datasheet numbers) and **8 extra, simulation-only inputs `DQ0_in` … `DQ7_in`** drawn on the left edge of the DIP outline, without pin numbers. In `mem` both are wired to the same net (`mem_d`), which is what the real pin does: the chip reads the net and drives it when reading. Inside: a #Memory RAM (17-bit address, separate data in / out, byte enables, trigger "high" = asynchronous level write: the cell follows DIN while the write is active, like the real /WE-controlled write) and gates for the truth table: write = CE# low · CE2 high · WE# low; output on = CE# low · CE2 high · OE# low · WE# high (a write disables the outputs, tWHZ). The power-up content is Logisim's (zero), the real chip's is random: no test may rely on it.

**M3 — the ROM model's contents.** A #Memory ROM keeps its contents as an attribute of the component inside the model circuit, so every instance of `at28c256` would hold the same data. The model therefore holds a **64 KiB ROM** and has one **extra, simulation-only input `HALF`** (no pin number): `mem` ties it to GND on the low chip and VCC on the high one, and the model reads image[HALF : A14..A0]. On the board, HALF is "which half was programmed into this chip". **Changed 2026-11-05 (cu-decisions.md Q10, section 14):** `HALF` became a 4-bit `IMAGE` input, the model holding 16 images of 32 KiB (0–1 the boot ROM, 2–8 the microcode EEPROMs, 9–15 erased); `mem` ties it to 0 and 1. `designs/at28c256.py --image file.bin` writes the image (default: a test pattern, every byte different between the halves); the boot test writes the image of the program under test, then restores the pattern. The model has no write path (/WE is tied high in `mem`; programming is the TL866's job); the datasheet's write modes are documented, not modelled, and the model's tests check that /WE low does nothing harmful (outputs as for /WE high, contents unchanged).

**M4 — a buffer between the ROMs and the RAM** (+1 74HCT541, mem 8 chips, 15 in all). The AT28C256 (Atmel 0006F, the copy in circuits/chips/) guarantees **VOH 2.4 V** (IOH −400 µA), and the AS6C1008 v1.0 asks **VIH 0.7 × VCC = 3.5 V**: the boot copy can't be shown correct with the ROM driving the RAM. The ROMs get their own net (`mem_rom_d`), always driven by exactly one of them (/CE = A15 / NOT A15, /OE tied low), and a third 74HCT541 (HCT input VIH 2.0 V, rail-to-rail output) drives it onto `mem_d` when `mem_rom_oe_n` = 0. Boot copy timing: clk → bus_addr 71 → 14 (32) → ROM tCE 150 → 541 (23) = **276 ns at 25 °C** vs 322 (margin 46); 85 °C: 309 vs 315 (6). The ROM's tOE no longer counts (always on). **Re-checked 2026-11-10 against AS6C1008 v1.2 and AT28C256 DS20006386B: kept** (section 11.3).

Datasheets in circuits/chips/: `as6c1008.pdf` (Alliance v1.0, Feb 2007, distributor copy: the current revision wasn't reachable from alliancememory.com tonight), `at28c256.pdf` (Atmel 0006F; Microchip's current DS20006386B wasn't downloadable: Mouser returned HTML). To replace with the current revisions (review list). **Replaced 2026-11-10** by v1.2 (March 2023) and DS20006386B (September 2022), section 11.

**M5 — the IOD stub is two tri-state buffers.** The bench's IOD stub (section 4) drives `bus_addr` low (the port) and `bus_data` (the data) through two 8-bit Controlled Buffers, enabled by bench pins; its interrupt queue, port values and timing live in the vector generator (`hbc2 vectors mem_cpu`), which takes them from the emulator's own IOD (`Iod::trigger_interrupt`, `Board::tick_iod`). A circuit FIFO would have checked Logisim's counters, not the CPU side; the stub's outputs go through the contention checks like any other driver. The register section is a stand-in too (`cpu_data` onto `bus_data`, `addr_bus_a` / `addr_bus_b` pins for R1:R2).

**M6 — the combined bench is its own file**, `circuits/mem/test_mem_cpu.circ` (circuit `test_mem_cpu`, made by designs/test_mem_cpu.py), loading addr, ir and mem as libraries: a module's own file can't instantiate itself, and the layout rule accepts a file holding only the circuit named after it (as `chips/test_74377.circ`). conventions.py warns that no module `mem_cpu` exists: expected.

**M7 — the combined bench's program is straight-line.** A random program with jumps lands mid-instruction and stops within a few hundred ticks; random STR / INC / DEC / NOT on memory overwrite the code; POF loads random flags (H); ASP / SSP / WSP move the stack anywhere. They are left out of `mem_cpu`'s program (every other opcode, every listed mode, invalid opcodes as NOPs: about 4,000 ticks, 54 hardware interrupts). PC changes of jumps, CAL, RET and IRT are covered by `addr_cpu`; the bench sets PC from the CPU's next PC through bus A:B after each instruction. INT is out because the pending port latch (C13) isn't built.

Phase 2 status: **done** (2026-11-05): M1–M7; tools, chip models, chips.md, `ir`, `addr_cout`, `mem`, `test_mem_cpu`, timing.py --section mem, control-requirements.md C6 / C18–C23, iod-requirements.md, B2; report: ir-mem-report.md. **Reviewed by Gianni** (section 10).

## 10. Gianni's review of the phase 2 report (2026-11-05)

Numbers are the items of ir-mem-report.md "Need Gianni's review".

1. **Simulation-only pins: accepted**, marked as such in conventions.md, ignored by bom.py and by any PCB netlist export. Done: `netlist.SIM_ONLY` lists them, `netlist.pcb_ports()` drops them, conventions.py checks each model against the list.
2. and 3. **Datasheets:** Gianni downloads the current AS6C1008 and AT28C256 datasheets into `circuits/chips/`. **Open:** when they are there, re-check M4 (the ROM buffer exists only for the AS6C1008 v1.0's VIH 0.7 × VCC; with VIH 2.2 V it goes, the ROMs drive the RAM directly) and the delays in chips.md, then rerun the tests and timing.py --section mem. **Done 2026-11-10** (section 11): VIH is still 0.7 × VCC, M4 stays, no delay changed.
4. and 5. **Accepted** (address setup before WE#, B2 measures it; `clk_mem_wr_n` global).
6. **Boot ROM image:** `hbc2 asm` / `hbc2 cc` `--boot-rom`, and the IDE's Build menu "Export boot ROM…", write the 64 KiB RAM image as two 32 KiB files, `<name>_lo.bin` (0000-7FFF, the AT28C256 with HALF = 0) and `<name>_hi.bin` (8000-FFFF, HALF = 1), each from the chip's address 0 (hbc2-core `boot_rom`; Documentation/HBC-2 Assembly Language.md "Boot ROM for the hardware CPU").
7.–12. No change asked.

## 11. Datasheet re-check (2026-11-10)

Gianni put the current datasheets in `circuits/chips/`. They replace the old copies under the same names (the folder's lowercase part-number naming):

| File | Was | Now |
|---|---|---|
| `as6c1008.pdf` | Alliance v1.0, February 2007 (distributor copy, 14 pages) | **Alliance v1.2, 13 March 2023** (15 pages). v1.1 (July 2021): "revise DC electrical characteristics"; v1.2: "revise P-DIP package outline dimension" |
| `at28c256.pdf` | Atmel 0006F | **Microchip DS20006386B, September 2022** (rev. A 2020: Microchip template; rev. B: ECC wording and TSOP removed). Its history also lists Atmel 0006 rev. M (2009), "updated AC characteristics and ordering information" |

### 11.1 What changed between the revisions

**AS6C1008** (v1.2 page numbers):

| Item | v1.0 | v1.2 | Matters to us? |
|---|---|---|---|
| VIH (p. 4) | 0.7 × VCC min, VCC + 0.3 max | same | **yes: 3.5 V at 5 V, M4 stays** |
| VIL (p. 4) | −0.2 to 0.6 V | same | no: HCT VOL is well under 0.6 V |
| VOH (p. 4) | 2.2 V min at −1 mA (typ 2.7) | **2.4 V** min at −1 mA | slightly better into the read buffer's HCT input (VIH 2.0) |
| VOL (p. 4) | 0.4 V at 2 mA | same | no |
| ISB1 C / I (p. 4) | 20 / 50 µA | 15 / 30 µA | no |
| ICC, ICC1 | 60 / 10 mA max | same | no |
| CIN, CI/O (p. 5) | 6, 8 pF max | same | no (timing.py counts 10 / 20) |
| AC test load (p. 5) | CL = 30 pF + 1 TTL | **CL = 50 pF + 1 TTL** | the same times now hold at a heavier load (section 4's "30 pF" reasoning is conservative) |
| Read (p. 5) | tRC 55, tAA 55, tACE 55, tOE 30, tCLZ 10, tOLZ 5, tCHZ 20, tOHZ 20, tOH 10 | same | — |
| Write (p. 5) | tWC 55, tAW 50, tCW 50, tAS 0, tWP 45, tWR 0, tDW 25, tDH 0, tOW 5, tWHZ 20 | same | — |
| Absolute max (p. 4) | VTERM −0.5 to 7.0 V | VCC −0.5 to 6.5 V, other pins −0.5 to VCC + 0.5 V | no (one 5 V supply) |
| Packages, pinout (p. 3) | SOP, P-DIP 600 mil, TSOP-I, sTSOP, TFBGA | same pinout; the P-DIP outline drawing is revised, its dimension table (0.600" rows, 0.100" pitch) reads the same | no |
| Ordering (p. 14) | no ordering table in the copy | -55PCN (PDIP, 0–70 °C), -55PIN, -55SIN, -55TIN, -55STIN, -55BIN | our **-55PCN** and the SOIC **-55SIN** are listed |
| NRND / obsolescence | none | none | — |

**AT28C256** (DS20006386B page numbers):

| Item | Atmel 0006F | DS20006386B | Matters to us? |
|---|---|---|---|
| VIH / VIL (p. 7) | 2.0 / 0.8 V | same | no: driven by HCT |
| VOH (p. 7) | 2.4 V min at −400 µA, no CMOS VOH row | same (`VOH1`, still no CMOS row) | **yes: confirms M4** |
| VOL (p. 7) | 0.45 V at 2.1 mA | same | no: into HCT inputs (VIL 0.8) |
| ILI, ILO, ISB1, ISB2, ICC | 10 µA, 10 µA, 200 µA (Mil 300), 3 mA, 50 mA | same, the military figure dropped | no |
| CIN, COUT (p. 8) | 4 / 6, 8 / 12 pF typ / max | same | no |
| Read, -15 (p. 11) | tACC 150, tCE 150, tOE 0–70, tDF 0–50, tOH 0 | same | — |
| Test load (p. 12) | 1.8 kΩ / 1.3 kΩ, 100 pF, inputs 0–3 V, 1.5 V reference | same | — |
| Write (p. 13) and page mode (p. 14) | tAS 0, tAH 50, tCS 0, tCH 0, tWP 100, tDS 50, tDH 0; tWC 10 ms (F: 3 ms), tBLC 150 µs max, tWPH 50 | same | no (the TL866 programs it) |
| Speeds, grades | -15 / -20 / -25 / -35; commercial, industrial, military | **-15 only, industrial only** (−40–85 °C, U = matte Sn) | our -15PU is industrial: the 85 °C column applies to the ROMs too |
| Packages (p. 4) | PDIP, SOIC, PLCC, TSOP, LCC, flatpack, CerDIP, PGA, die | **PLCC-32, PDIP-28, SOIC-28** | no: PDIP-28 pinout unchanged |
| Ordering (p. 29) | -15PC / -15PI … | AT28C256-15PU (28P6), -15SU, -15JU; the E and F options in JU / SU only | our **-15PU** and **-15SU** are listed |
| NRND / obsolescence | none | none | — |

### 11.2 Every value we use, against the new datasheets

| Value | Where | Assumed | Datasheet | |
|---|---|---|---|---|
| RAM tAA 55 | chips.md; timing.py m_read → m_fetch, c_bypass, c_next, x_mem_reg, x_fetch, x_bypass | 55 | 55 (p. 5) | ✓ |
| RAM tACE 55 | chips.md; m_read_ce | 55 | 55 | ✓ |
| RAM tOE 30 | chips.md (in no path: the read buffer's /OE is later) | 30 | 30 | ✓ |
| RAM tWP 45 | chips.md; m_pulse limit; B2 | 45 | 45 | ✓ |
| RAM tDW 25 | chips.md; m_wdl (limit of m_wdata, m_boot, x_wdata) | 25 | 25 | ✓ |
| RAM tAS 0 | m_addr (limit: WE# falls, earliest); M1; B2 | 0 | 0 | ✓ |
| RAM tWR 0, tDH 0 | m_rise | 0 | 0 | ✓ |
| RAM tAW 50 | chips.md note (address valid by 72, WE# rises from 347 at 25 °C: 275 ns) | 50 | 50 | ✓ |
| RAM tWHZ 20, tOHZ 20 | section 5 list; the model turns DQ off while WE# is low | 20 | 20 | ✓ |
| RAM VIH | M4; section 5 "Levels" | 0.7 × VCC = 3.5 V | 0.7 × VCC (p. 4) | ✓ |
| RAM VOL 0.4 V at 2 mA, test load 30 pF | section 4 (why the read buffer) and section 5 (`mem_d`) | 30 pF | **50 pF** (p. 5) | better; the read buffer is still needed (`bus_data` is about 386 pF) |
| RAM CIN 6, CI/O 8 pF | timing.py PIN_PF (10 per input, 20 per 3-state output, every chip) | ≤ 10 / ≤ 20 | 6 / 8 (p. 5) | ✓ conservative |
| AS6C1008-55PCN, -55SIN | chips.md, bom-mem.csv, bom-cpu.csv | listed | p. 14 | ✓ |
| RAM PDIP-32 pinout | `as6c1008` model | — | p. 3 | ✓ unchanged |
| ROM tACC 150 | chips.md; c_step, c_ir, c_bypass, c_next, c_force, c_jc, c_int, c_state, c_sel, x_flag_jc; cu-decisions.md section 6 | 150 | 150 (p. 11) | ✓ |
| ROM tCE 150 | chips.md; m_boot; M4 | 150 | 150 | ✓ |
| ROM tOE 70 | chips.md (in no path: /OE tied low since M4) | 70 | 70 | ✓ |
| ROM tOH 0 | cu-decisions.md section 6 (hold times) | 0 | 0 | ✓ |
| ROM tDF 50 | not used | — | 50 max (p. 11) | see 11.4 |
| ROM VOH | M4 | 2.4 V at −400 µA | 2.4 V (p. 7) | ✓ |
| ROM VOL, VIH, VIL | microcode outputs into HCT; ROM inputs from HCT | 0.45 / 2.0 / 0.8 | same | ✓ |
| ROM CIN 6, COUT 12 pF | PIN_PF on `cu_a`, `bus_addr`, `mem_rom_d` | ≤ 10 / ≤ 20 | 6 / 12 (p. 8) | ✓ |
| ROM test load 100 pF | section 5 (`mem_d`) | 100 | 100 (p. 12) | ✓ |
| AT28C256-15PU, -15SU | chips.md, bom-mem.csv, bom-cpu.csv, cu-decisions.md | listed | p. 29 | ✓ |
| ROM PDIP-28 pinout | `at28c256` model | — | p. 4 | ✓ unchanged |

No figure changed: chips.md's Delays keep their values (only the source column now names the new revisions), and timing.py computes exactly what it did.

### 11.3 M4: kept

The AS6C1008 v1.2 still asks **VIH ≥ 0.7 × VCC = 3.5 V** (page 4, DC Electrical Characteristics, unchanged since v1.0), and the AT28C256 DS20006386B still guarantees only **VOH ≥ 2.4 V at −400 µA** (page 7, Table 4-2; no CMOS VOH figure). The ROMs can't drive the RAM on paper, so the ROM buffer (third 74HCT541, `mem_rom_d` → `mem_d`) stays and the circuit doesn't change. The "VIH 2.2 V" of phase 2 doesn't exist in either revision; the 2.2 V figure in v1.0 is VOH, which v1.1 raised to 2.4 V.

For the record, what removing it would have bought: the boot copy loses the 541 (23 / 29 ns), 41 → 64 ns of margin at 25 °C (50 % duty). The tightest memory path, the address on `bus_addr` before WE# falls (25 ns at 25 °C, 12.5 at 45 % duty), doesn't go through the ROM buffer: M4 has no bearing on it.

### 11.4 Other observations

- **Two ROM outputs on `mem_rom_d` at each A15 change (not modelled, not yet decided).** Both ROMs have /OE tied low and /CE = A15 / NOT A15, so the one being deselected floats only within tDF ≤ 50 ns while the other may turn on sooner (no minimum is given for /CE → output on). Every A15 change on `bus_addr` can give up to about 50 ns of overlap, in normal running too, not just in the boot. The data isn't used then (the ROM buffer is off outside the boot, and the boot copy samples 150 ns later), so it is current between two weak outputs, not a logic error. Logisim switches /CE instantly, so cputest's contention check can't see it. A possible fix, for Gianni: drive both ROMs' /OE from `mem_rom_oe_n`, which turns them off outside the boot (tOE 70 then counts again, but 33 + 70 is well under the tCE path); the overlap would remain only at the boot's 7FFF → 8000 step.
- **The IOD's levels.** `bus_addr` reaches the RAM's address pins directly. In the vector step the RAM isn't accessed, so an IOD with TTL-level outputs would work, but iod-requirements.md I9 now recommends rail-to-rail (HCT or CMOS) outputs on both buses.
- **RAM grade.** The -55PCN (0–70 °C) is still what we use; the ROMs are now industrial only (-15PU), and the RAM's industrial -55PIN is listed on page 14.

### 11.5 What was done

- `circuits/chips/as6c1008.pdf` and `at28c256.pdf` are the new revisions; the old copies are deleted.
- chips.md: the Delays table's source column (with page numbers) and the memories paragraph name the new revisions.
- `designs/as6c1008.py`, `designs/at28c256.py`: docstrings and the label on the model now name v1.2 and DS20006386B; the models regenerate otherwise byte-identical; `--test` passes (3,260 and 302,912 rows).
- timing.py: memory part names were cut to "008" / "256" in the printed paths (`part[5:]` assumed 74HCT); they print whole now. Every number is unchanged.
- No circuit changed, so the phase 4 CPU comparisons were not rerun.

timing.py, every section, worst margins (ns, max 25 °C pass / max 85 °C information):

| Section | 45 % duty | 50 % duty | 55 % duty |
|---|---|---|---|
| flags | 208 / 134 | 208 / 134 | 208 / 134 |
| regs | 16.5 / −27 | 29 / −27 | 41.5 / −27 |
| addr | 79 / −32 | 79 / −32 | 79 / −32 |
| mem | 12.5 / −14.5 | 25 / −2 | 36.5 / 10.5 |
| cu | 12.5 / −14.5 | 25 / −2 | 36.5 / −2 |
| cpu | 25 / −36 | 25 / −36 | 25 / −36 |

All limits are met at 25 °C at every duty. The memory paths (25 / 85 °C margins at 45 / 50 / 55 %): address before WE# falls 12.5 / −14.5, 25 / −2, 37.5 / 10.5; RAM write data 65.5 / −9.5, 78 / 3, 90.5 / 15.5; boot copy (with the ROM buffer) 28.5 / −13.5, 41 / −1, 53.5 / 11.5; RAM read → `bus_data` 136.5 / 89.5, 149 / 102, 161.5 / 114.5; WE# pulse 129 / 110 at every duty. The EEPROM paths don't depend on the duty: the bypass 61 / −2, the INT boundary 41 / 0, the step register 239 / 211.
