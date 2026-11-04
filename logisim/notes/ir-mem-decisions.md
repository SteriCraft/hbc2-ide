# Instruction register and memory / bus interface: decisions

**Phase 1 (paper design, 2026-11-05).** No circuits yet. Waiting for Gianni's go-ahead (questions in section 6).

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

Phase 2 status: M1 decided (commit 775f21b). Found: netlist.py / circgen.py have no port geometry for #Memory RAM / ROM yet (only Register, D Flip-Flop): add it first, from the Logisim-evolution jar in `C:/Program Files/logisim-evolution/app` (std/memory Ram / Rom port layout and attributes), checked by `logitest.opens`. Next: chip models as6c1008 + at28c256 (designs/as6c1008.py, at28c256.py), then ir, addr_cout, mem, benches, timing.py, requirement notes, report (section 8).
