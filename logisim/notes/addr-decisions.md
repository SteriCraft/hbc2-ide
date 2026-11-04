# Address section (PC, SP, address adder, address latch): decisions

**Phase 2 done (night of 2026-11-04, unattended).** Module `addr` in `circuits/addr/addr.circ`, made by `designs/addr.py`: 34 chips, tests 31,640 / 31,640 (`hbc2 vectors addr`, model) and 65,114 / 65,114 (`addr_cpu`, the real CPU). Summary for Gianni: [addr-report.md](addr-report.md). Phase 2 decisions: section 4 (A1–A9). Sections 1–3 are the phase 1 design, updated where phase 2 changed it.

Source of truth: `hbc2-core/src/cpu.rs` (and `isa.rs`, `board.rs`). Requirements met here: [pc-sp-requirements.md](pc-sp-requirements.md) P1–P4. Out of scope (ports only): the IR, memory, the IOD, the control unit.

## 1. Every use of PC, SP and the memory address (from cpu.rs)

Every 16-bit sum wraps. n = instruction size (1–4). Vx = V1:V2 (V1 high). "push x" = SP := SP − 1, then mem[SP] := x. "pop" = read mem[SP], then SP := SP + 1.

| Instruction / event | Mode | Memory or I/O address | PC update | SP update |
|---|---|---|---|---|
| fetch (every instruction) | | PC, PC+1, … PC+n−1 | PC := PC + n | |
| fetch, invalid mode (`stops()`) | | PC (+1) | **unchanged** (stays on the instruction) | |
| ADD ADC SUB SBB AND OR XOR, LOD, STR | RegRam | Vx | | |
| same + CMP | mode 4 (RamReg/ImmReg) | R1:R2 + V2 (V2 zero-extended) | | |
| same + CMP | StackOffset | SP + V1 (V1 zero-extended) | | |
| INC DEC NOT | Imm16 | Vx, read then write | | |
| INC DEC | mode 4 | R1:R2 + V2, read then write | | |
| PSH, PSF | Reg / None | SP − 1 (write) | | SP − 1 |
| POP, POF | Reg / None | SP (read) | | SP + 1 |
| CAL | Imm16 / Reg16 | SP − 1 ← PC lo, SP − 2 ← PC hi | PC := Vx / R1:R2 | SP − 2 |
| RET | None | SP (hi), SP + 1 (lo) | PC := hi:lo | SP + 2 |
| IRT | None | SP (hi), SP + 1 (lo), SP + 2 (I) | PC := hi:lo | SP + 3 |
| JMP JMC JME JMN JMS JMZ JMF | Imm16 / Reg16 | | if taken: PC := Vx / R1:R2 | |
| ASP / SSP | Imm8 | | | SP ± V1 |
| ASP / SSP | Imm16 | | | SP ± Vx |
| WSP | Reg16 / Imm16 | | | SP := R1:R2 / Vx |
| RSP | Reg16 | | | SP → R1 (hi), R2 (lo), through the data bus (P3) |
| IN / OUT | Reg | I/O 00:R2 / 00:R1 (P1) | | |
| INT | Imm8 | bus_addr = 00:V1, bus_data = I; **(V1, I) kept pending** | | |
| interrupt entry, hardware | | SP − 1 ← I, SP − 2 ← PC lo, SP − 3 ← PC hi; port p from bus_addr (the IOD drives 00:p); mem[FE00 + 2p] (hi), mem[FE00 + 2p + 1] (lo) | PC := hi:lo | SP − 3 |
| interrupt entry, software | | the same, p and the data from the pending INT | PC := hi:lo | SP − 3 |
| reset | | | PC := 0000 | SP := FE00 |

Nothing else touches PC, SP or the address (NOP, ALU Reg / RegImm8 modes, shifts, MOV, flag set/clear, HLT, untaken jumps).

What follows:
- One adder does it all: **base + offset**, with the offset inverted for subtraction. Bases: PC, SP, R1:R2, the IVT entry; offsets: 0, V1, V2, Vx, ±1. SSP Imm16 needs a full 16-bit subtraction.
- FE00 + 2p is wiring: high byte = `1111111 p7`, low byte = `p6..p0 0`. No adder needed for the vector itself.
- A software interrupt stays pending across instructions (CLI, or a hardware interrupt goes first): its port must be stored somewhere (Q1; the data is I when the interrupt is taken).

## 2. Block diagram

```
 PC ─────┐
 SP ─────┤ BASE: 4:1 per byte, 8 × 153            OFFSET: hi = V1 | 00 (2 × 157)
 regs_a:regs_b ┤ /E hi, /E lo = 00                        lo = V1 | V2 | 00 (2 × 157)
 IVT(bus_addr lo) ┘ (1111111 a7 : a6..a0 0)                    │
        │                                       XOR addr_inv (4 × 86)
        └──────────────── A        4 × 283        B ───────┘
                                      │  Cin = addr_cin
                                   addr_sum (16)
               ┌──────────────────────┼───────────────────────┐
     address latch 2 × 377       PC 2 × 377              SP 2 × 377
     (load enable)               (load enable)           (load enable)
        │                          │   └→ BASE              │   └→ BASE
     2 × 541, /OE hi, /OE lo     2 × 541 → bus_data      2 × 541 → bus_data
        │                          (PC hi, PC lo)          (SP hi, SP lo)
     bus_addr (16) ──→ memory, IOD, and back to BASE (IVT, low byte only)
```

- **Everything goes through the adder** and lands on `addr_sum`; the latch, PC and SP each load it when their enable is on (74HCT377: edge-triggered on `clk`, holds otherwise, no gated clock: P1).
- **The latch is the only CPU driver of `bus_addr`**, through two 541s with a /OE per byte: off while the IOD drives the port (interrupt acknowledge), low byte off while the pending port latch drives it (software vector, Q1).
- **PC and SP never load from the data bus.** RET, IRT and the IVT read put their two bytes in the IR's V1 and V2 latches, then PC := 0 + Vx (Q4). That removes 4 × 157 at PC's inputs and 4 inputs from the data bus.
- **The same sum feeds the latch and PC (or SP) in one step**, which pipelines sequential accesses: "latch, PC := PC + 1" while memory reads at the old latch value. The settings:

| Formula | BASE (sel, enables) | OFFSET | inv | Cin |
|---|---|---|---|---|
| PC, PC + 1, PC − 1 | PC | 0 | 0 / 0 / 1 | 0 / 1 / 0 |
| SP, SP + 1, SP − 1 | SP | 0 | 0 / 0 / 1 | 0 / 1 / 0 |
| SP + V1, SP − V1 | SP | 00:V1 | 0 / 1 | 0 / 1 |
| SP + Vx, SP − Vx | SP | V1:V2 | 0 / 1 | 0 / 1 |
| R1:R2 + V2 | R1:R2 | 00:V2 | 0 | 0 |
| R1:R2 (jump, WSP) | R1:R2 | 0 | 0 | 0 |
| 00:R (IN, OUT) | R1:R2, high byte off | 0 | 0 | 0 |
| Vx, 00:V1 (INT) | off | V1:V2 / 00:V1 | 0 | 0 |
| FE00 + 2p | IVT(bus_addr lo) | 0 | 0 | 0 |
| 0000 (reset) | off | 0 | 0 | 0 |

Step sequences (one line per clock step; "latch :=" takes effect at the end of the step, the access uses the latch loaded in the step before):

| Sequence | Steps |
|---|---|
| fetch, n bytes | latch := PC (usually merged into the previous instruction's last step); n × (read byte → IR; latch, PC := PC + 1). After it PC = PC + n and the latch already points at the next instruction |
| fetch, invalid mode | as above, then PC := PC − 1 once per byte read (1 or 2 steps) before halting (Q5) |
| PSH / PSF | latch, SP := SP − 1; write |
| POP / POF | latch := SP; read, SP := SP + 1 |
| RegRam / mode 4 / StackOffset | latch := Vx / R1:R2 + V2 / SP + V1; access (P1: the registers are free from then on) |
| CAL | latch, SP := SP − 1; write PC lo, latch, SP := SP − 1; write PC hi, latch, PC := target (the next fetch starts) |
| RET | latch := SP; read → V1, latch, SP := SP + 1; read → V2, SP := SP + 1; latch, PC := Vx |
| IRT | as RET, with a third read (→ I) and SP + 1 before latch, PC := Vx |
| JMx taken | latch, PC := Vx / R1:R2 |
| interrupt entry | latch, SP := SP − 1; write I, latch, SP := SP − 1; write PC lo, latch, SP := SP − 1; write PC hi (latch := 0000); INR (hardware) and wait for INT low; **vector step**: latch outputs off (hardware) or low byte off (software), bus_addr = 00:p, I := bus_data (hardware only: for software the data is I itself, so I is not written), latch, PC := FE00 + 2p; read → V1, latch := PC + 1; read → V2; latch, PC := Vx |
| reset | latch, PC := 0000; SP := IVT(bus_addr lo = 00) + 0 = FE00 (inside the reset sweep, C7) |

## 3. Chips, timing, open questions

### Chips (34: phase 1's 32 + 2 × 74HCT08 for the reset, A1)

| Chips | Part | Role |
|---|---|---|
| 8 | 74HCT153 | BASE multiplexer: PC / SP / R1:R2 / IVT, /E per byte gives 00 |
| 4 | 74HCT157 | OFFSET: high byte V1 or 00 (/G), low byte V1 or V2 or 00 (/G) |
| 4 | 74HCT86 | offset inversion (subtraction, −1) |
| 4 | 74HCT283 | 16-bit adder, ripple carry |
| 2 + 2 | 74HCT377 + 74HCT541 | address latch + its bus_addr drivers (/OE per byte) |
| 2 + 2 | 74HCT377 + 74HCT541 | PC + its bus_data drivers (hi, lo) |
| 2 + 2 | 74HCT377 + 74HCT541 | SP + its bus_data drivers (hi, lo, P3) |
| 2 | 74HCT08 | reset: PC bits 15..9 and the IVT's p7 AND `rst_n` (A1) |

Every part is in Logisim's #TTL (chips.md). New part: **74HCT377**: CD74HCT377E (DIP) and CD74HCT377M96 (SOIC), both active in TI's SCHS184C addendum. Its datasheet delays (CP → Q 38 / 48 ns, setup 12 / 15 ns) replace phase 1's 574 placeholders (A4). Logisim's 74377 was checked against the datasheet (A5).

### Timing: phase 1 estimate (the built circuit's figures are in section 4, A4)

| Path | Through | 25 °C | 85 °C | Limit | Margin 25 / 85 |
|---|---|---|---|---|---|
| control → OFFSET → sum → latch / PC / SP setup | CU 574 (33 / 41) → 157 S→Y (29 / 36) → 86 (32 / 40) → 283 A→C4 (48 / 60) → 2 × Cin→C4 (92 / 116) → Cin→S3 (53 / 66) → 377 setup (12 / 15) | 299 | 374 | 500 | 201 / 126 |
| **regs_a / regs_b → sum → setup** (Reg16, mode 4, IN/OUT) | operands valid (153 / 195, P2) → 153 I→Y (34 / 43) → 283 A→C4 → 2 × Cin→C4 → Cin→S3 (193 / 242) → setup (12 / 15) | 392 | 495 | 500 | **108 / 5** |
| IOD port on bus_addr → IVT → setup (vector step) | 153 I→Y (34 / 43) → 283 chain (193 / 242, counted as a full ripple) → setup (12 / 15) | 239 | 300 | 500 | the IOD must drive bus_addr by **261 / 200 ns** |
| latch → bus_addr valid | 377 CP→Q (33 / 41) → 541 A→Y (23 / 29) + load (~146 pF: +10 / +12) | 66 | 82 | — | memory gets the rest |
| PC / SP → bus_data (CAL, interrupt entry, RSP) | CU 574 (33 / 41) → 541 OE→Y (30 / 38) + data bus load (+37 / +49) | 100 | 128 | 335 / 325 (C10, P3) | 235 / 197 |

Bus loading from this section: `regs_a` and `regs_b` get 1 input per bit (the BASE 153), as P2 assumed. `bus_data` gets 4 three-state outputs (PC, SP) and **no inputs**: timing.py's estimate (2 in + 2 out each for PC and SP) drops by 4 inputs. `bus_addr` (low byte, worst): 3 outputs (latch, IOD, software vector) and about 5 inputs (memory, IOD, BASE, the software latch). Every path passes at 25 °C; the register path has 5 ns left at 85 °C (information only).

### Phase 1 questions: all answered (Gianni, 2026-11-04: Q1 done, Q2–Q6 accepted as recommended)

- **Q1. Pending software interrupt. Decided (Gianni, 2026-11-04): keep the port only.** A pending port latch in the control unit, 1 × 377 + 1 × 541, loaded while INT puts 00:V1 on bus_addr and driving the low byte back in the vector step (the address latch drives 00 on the high byte). The data is I when the interrupt is taken, so the vector step does not write I, and the pushed I is that same value. The 4-chip option (port and data, like the IOD) was dropped.
  - **Emulator changed to match** (cpu.rs: `pending_software_interrupt` is `Option<u8>`, the port; I is read on entry; the IDE's CPU view shows the port only). Before, cpu.rs kept (V1, I) from INT, and a handler saw the old I when I changed while the interrupt was pending.
  - Test: `pending_software_interrupt_takes_i_when_taken` (hbc2-core/tests/interrupts.rs): CLI, I := 11, INT 10, I := 22, STI → the handler gets I = 22, 22 is pushed, IRT restores 22.
  - Checks: the emulator test suite passes, except `hbc2-cc` `error_lines`, which fails the same way without this change (the uncommitted compiler/fcc work). All 11 `hbc2 vectors` modules are byte-identical before and after (none uses INT), so no vectors needed regenerating. The section tests (alu, flags, regs) were re-run on the existing circuits.
- **Q2. Interrupt entry order.** **Recommended:** push I and PC *before* raising INR, then take the port (bus_addr) and the data (→ I) in the vector step. Memory, registers and the wire protocol (INR one step, the IOD's reply one step) are the same as the manual's order; the manual's order would need the port and the data held for 3 steps (2–4 more chips).
- **Q3. PC and SP onto the data bus.** **Recommended:** separate drivers (4 × 541). Sharing 2 × 541 behind the BASE multiplexer saves 2 chips but CAL and interrupt entry take 2 more steps (the multiplexer can't show PC while it computes SP − 1); same reasoning as registers D1 (c).
- **Q4. RET / IRT / IVT bytes through the IR's V1 and V2.** **Recommended** (saves 4 × 157 and 4 data bus inputs, costs 1 step on RET, IRT and entry). Needs the IR to load V1 and V2 from the data bus in any step.
- **Q5. Invalid instruction.** **Recommended:** PC is incremented during the fetch; for `stops()` the control unit undoes it with PC − 1 (1 or 2 steps, then the CPU halts). Alternative: a fetch offset counter feeding OFFSET (more chips, every fetch).
- **Q6. 377 or counters for PC.** **Recommended:** 377 (PC + 1 goes through the adder with the latch, one step per byte). 4 × 74HCT161 would also count but adds 2 chips and still needs the adder for the other loads.

### For the control unit (copied to control-requirements.md in phase 2, plus C16 for the reset)

- C11: drive the section's controls from the clocked control register (BASE sel + 2 enables, OFFSET lo sel + 2 enables, inv, Cin, 3 load enables, 6 output enables). At most one driver on `bus_addr` (latch per byte, IOD, software vector). P4 applies (no load from `regs_a`/`regs_b` while `regs_we_n` is low).
- C12: the interrupt entry sequence above (Q2), including the vector step: latch outputs off, RAM and IOD not enabled by the CPU, I written from `bus_data` (hardware interrupts only).
- C13: the pending software interrupt port latch (Q1): 1 × 377 + 1 × 541, loaded by INT (00:V1 on bus_addr), cleared when taken, drives bus_addr low byte in the software vector step; I is not written there.
- C14: the IR loads V1 and V2 from `bus_data` in any step (Q4).
- C15: undo the fetch increments on an invalid instruction (Q5).

## 4. Phase 2 (night of 2026-11-04): decisions taken alone

Gianni was away; per his instructions, each question got the answer I'd recommend, logged here. What he should look at is in [addr-report.md](addr-report.md), "Need Gianni's review".

### A1. PC := 0000 and SP := FE00 from the reset line

Gianni: the 377 has no clear or preset, but the reset values must come from `rst_n`, not from a sequence; pick the cheapest compliant option and log why.

PC and SP load the **same** sum, so forcing the sum gives only one of the two values; something must differ on one register's D inputs. Options counted:

| Option | Chips added | Notes |
|---|---|---|
| **(chosen)** While `rst_n` is low the control unit holds a **reset word**: BASE = IVT, high byte only (`addr_base_sel` = 11, `addr_base_oe_n` = 01), no offset, `inv` = `cin` = 0, PC and SP loaded, every driver off. The sum is the IVT's high byte, 1111111 p7 : 00. **2 × 74HCT08**: p7 AND `rst_n` (so the sum is FE00 whatever the bus shows), and PC's D bits 15..9 = sum AND `rst_n` (so PC takes 0000 from the same sum). SP := FE00 and PC := 0000 on every `clk` edge while `rst_n` is low. | **+2** | Uses `rst_n` directly: no inverter, no new global. The control unit must have a defined control word while `rst_n` is low anyway (otherwise the buses contend at power-up), so the word costs it nothing (C16). +24 / 30 ns on PC loads, included in A4. |
| The same with the sum forced to 0000 and SP's bits 15..9 ORed with `rst` | +2, plus an inverter | Needs `rst` active high: a new global or an inverter. |
| Self-contained: the forcing gates inside `addr` (BASE and OFFSET enables off, inv and cin 0, PC and SP loads on), plus the 7 SP gates | +5 or 6 (3 × 32, 2 × 08, an inverter) | Doesn't depend on the control unit's reset word. **The fallback if Gianni prefers it.** |
| PC as 4 × 74HCT161 (asynchronous clear, synchronous load, hold) | +2 | Solves PC only: SP still needs FE00, so the forcing is needed anyway. |
| PC as 74HCT273 (clear) | — | No load enable: PC would need a feedback multiplexer (P1 forbids a gated clock). |

The reset is **synchronous**: it needs at least one `clk` edge while `rst_n` is low (the clock runs from the oscillator during reset), and the **release of `rst_n` must be synchronous to `clk`** (a flip-flop in the reset generator), since the control word and the 08s are sampled at the edge. The address latch is not loaded by the reset; the first fetch starts with "latch := PC" (C16).

Tested: `addr` resets from power-up and from random states with p7 = 0 and 1 on the bus; every `addr_cpu` sequence ends with a reset, checked against the emulator's `Cpu::new()` (PC = PROGRAM_START, SP = STACK_RESET). A mutant without the p7 gate fails both tests (173 and 5,764 rows).

### A2. Interface (the module's pins)

| Pin | Width | Meaning |
|---|---|---|
| `clk`, `rst_n` | 1 | global |
| `addr_base_sel` | 2 | BASE: 00 PC, 01 SP, 10 bus A : bus B (R1:R2), 11 IVT (1111111 p7 : p6..p0 0) |
| `addr_base_oe_n` | 2 | BASE byte enables (bit 1 high, bit 0 low; off = 00), the 153s' /E |
| `addr_off_sel` | 1 | OFFSET low byte: 0 = V2, 1 = V1 (the high byte is always V1) |
| `addr_off_oe_n` | 2 | OFFSET byte enables (off = 00), the 157s' /G |
| `addr_inv`, `addr_cin` | 1 | invert the offset; adder carry in (`inv` + `cin` = subtract; `inv` alone with offset 0 = −1) |
| `addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n` | 1 | 377 /E: load the sum on the next `clk` edge |
| `addr_lat_oe_n` | 2 | latch onto the address bus, per byte (541 /OE) |
| `addr_pc_oe_n`, `addr_sp_oe_n` | 2 | PC / SP byte onto the data bus (bit 1 high, bit 0 low); at most one of the four low |
| `addr_bus_a`, `addr_bus_b` | 8 | `regs_a` (R1, high byte), `regs_b` (R2, low byte), P2 |
| `addr_v1`, `addr_v2` | 8 | the IR's V1 and V2 |
| `addr_bus_addr_lo_in` | 8 | the address bus's low byte, read back (IVT: the IOD's or the pending latch's port) |
| `addr_bus_addr_hi`, `addr_bus_addr_lo` | 8 | out, tri-state: the latch onto the address bus |
| `addr_bus_out` | 8 | out, tri-state: PC / SP onto the data bus |

The address bus is **two 8-bit pins** instead of one 16-bit `bus_addr`: each byte has its own enable (the software vector step drives the high byte only), and a Logisim test vector can't expect a half-floating 16-bit value. At CPU level a splitter joins them into `bus_addr`. `addr_bus_addr_lo_in` is the same bus's low byte as an input, like `regs_bus_in` / `regs_bus_out`.

### A3. OFFSET high byte

A 74HCT157 with SELECT and the B inputs tied to Ground, used as "V1 or 00" through its /G. Two 74HCT08s would do the same; the 157 keeps the two bytes alike. Same chip count.

### A4. 74HCT377 timings (TI SCHS184C) and the section's timing

The 377 is slower than the 574 placeholder: CP → Q 38 / 48 ns (574: 33 / 41); setup 12 / 15 ns (the same). Built-circuit figures (`python tools/timing.py --section addr`, bus loading counted on the circuits):

| Path | Max 25 °C | Max 85 °C | Limit | Margin 25 °C (pass) / 85 °C (info) |
|---|---|---|---|---|
| control (OFFSET select) → 157 → 86 → 4 × 283 → 08 → PC setup | 323 | 404 | 500 | 177 / 96 |
| control (BASE select) → 153 → adder → PC setup | 296 | 371 | 500 | 204 / 129 |
| V1 / V2 (IR) → adder → PC setup | 319 | 399 | 500 | 181 / 101 |
| PC / SP (377) → BASE → adder → PC setup (PC + 1, SP − 1) | 301 | 378 | 500 | 199 / 122 |
| **regs_a / regs_b → BASE → adder → PC setup** (Reg16, mode 4, IN / OUT) | **416** | **525** | 500 | **84 / −25** |
| bus_addr low byte → IVT → adder → PC setup | 263 | 330 | 500 | the IOD must drive the bus by **237 / 170 ns** |
| clk → latch → bus_addr valid | 71 | 90 | 335 / 325 (C10) | 264 / 235: what memory gets to put its byte on the data bus |
| control → latch drivers on | 73 | 92 | 335 / 325 | 262 / 233 |
| PC / SP → bus_data | 96 | 121 | 335 / 325 (C10, P3) | 239 / 204 |

Every path passes at 25 °C. The register path is 25 ns over at 85 °C (information only, like the register section's end-to-end path). If the hardware shows errors there, the chips.md fallback applies: 74ACT153 for the 8 BASE multiplexers (I → Y 16 ns instead of 43 at 85 °C) gives back about 27 ns.

Loads are now counted on the circuit instead of estimated: `bus_a` / `bus_b` take one 153 input per bit (as P2 assumed); `bus_data` takes 4 tri-state outputs and **no input** (phase 1's estimate had 4 more inputs): 372 pF instead of about 420. `bus_addr` (low byte, worst bit): 146 pF, with an estimate for memory, the IOD and the pending port latch. The other sections still pass (`timing.py --section alu / flags / regs`).

Hold times: 377 D hold 3 ns, /E hold 5 ns after the edge. The sum can't change that fast (it goes through a 377 or 574 and the 153 / 283 chain), but the load enables come straight from the control register, and TI gives no minimum CP → Q for the 574. Review item.

### A5. Logisim's 74377 checked

`designs/test_74377.py` → `circuits/chips/test_74377.circ`: Logisim's chip against the datasheet's truth table, 250 / 250 rows, including /E moving while CP is high (no load: /E is sampled at the edge, not a clock gate). No model needed. The datasheet is `circuits/chips/cd74hct377.pdf`.

### A6. Phase 1 answers applied

Q1 was done by Gianni (emulator). Q2 (push before INR, port in the vector step), Q3 (separate PC / SP drivers), Q4 (RET / IRT / IVT bytes through V1 and V2), Q5 (undo the fetch increments on an invalid instruction) and Q6 (377, not counters) are built and tested as recommended.

### A7. Tests

- `addr` (model, `hbc2/crates/hbc2-cli/src/vectors/addr.rs`): the reset (32 times, p7 = 0 / 1); all 512 combinations of BASE select × BASE enables × OFFSET enables × OFFSET select × inv × cin with random data (4 passes), loaded into the latch and alternately PC / SP, read back; every load and output enable on its own (the others hold); 40 random sequences of 200 steps, `rst_n` low now and then. 31,640 rows.
- `addr_cpu` (the real CPU): 48 (PC, SP) pairs, 6 of them corners (PC = FFFF / FFFE wrapping, SP = 0000 / FFFF / FE00, SP near the IVT), × 40 events: every addressing mode that touches memory (Vx, R1:R2 + V2, SP + V1, read-modify-write), PSH / POP / PSF / POF, CAL / RET / IRT, taken and untaken jumps, ASP / SSP / WSP / RSP, IN / OUT, HLT, NOP, MOV, INT + the software entry, a hardware entry through the IOD handshake (random port, p ≥ 128 included), an invalid instruction. Checked against the CPU: the address of every fetched byte, every operand address, every pushed PC byte, PC and SP at the end, then the reset (its RAM writes through a `Probe`, `board.address_bus` for I/O and INT, the loaded bytes for reads). 65,114 rows.
- Mutation check: removing the p7 gate, or breaking the carry between the second and third 283, fails both tests.
- Not covered (other sections): the pending port latch (C13), INR and the IOD's timing, memory.

### A8. Steps the bench uses (for the control unit)

Section 2's step table, as tested. Two details found while writing it:
- Interrupt entry: the last push step also does "latch := 0000", so the latch shows 00 on the high byte during the software vector step (low byte off, the pending latch drives p). The hardware entry does the same (harmless: the latch's outputs are off then).
- RET / IRT: the return address's high byte goes to V1 at the end of the step that reads it, the low byte to V2 one step later; "latch, PC := Vx" comes after both.

### A9. Not done in phase 2

- The control unit, the IR, memory, the IOD and the pending port latch are other sections (ports only here).
- `cpu.circ` doesn't instantiate `addr` yet (nor the other sections); `tools/usages.py addr` finds no user, so nothing needs rewiring.

## 5. Gianni's review (2026-11-05)

All 8 items of addr-report.md's review list accepted as recommended:

1. The reset relies on the control unit's reset word (C16), 2 × 74HCT08; the self-contained version stays unused.
2. `rst_n` released synchronously to `clk`: the reset generator is now control-requirements.md **C17** (RC + 74HCT14 + 74HCT74, two-stage synchronizer, asynchronous assert).
3. The register-operand path's −25 ns at 85 °C is accepted (criterion max 25 °C); 74ACT153 for the BASE multiplexers stays the fallback.
4. The 377 hold times (/E 5 ns, D 3 ns): **to check on the breadboard with a scope** ([breadboard-checks.md](breadboard-checks.md) B1).
5. The address bus as two 8-bit pins plus `addr_bus_addr_lo_in` (A2): kept.
6. The 74HCT377 references: kept (CD74HCT377E, CD74HCT377M96).
7. 34 chips: accepted.
8. The steps of A8: accepted, for the control unit.
