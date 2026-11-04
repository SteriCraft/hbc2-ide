# Address section (PC, SP, address adder, address latch): decisions

**Phase 1, on paper (2026-11-04): waiting for Gianni's go-ahead.** No circuit yet. Planned module: `addr` in `circuits/addr/addr.circ`, made by `designs/addr.py`, one module like `regs`.

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

### Chips (32)

| Chips | Part | Role |
|---|---|---|
| 8 | 74HCT153 | BASE multiplexer: PC / SP / R1:R2 / IVT, /E per byte gives 00 |
| 4 | 74HCT157 | OFFSET: high byte V1 or 00 (/G), low byte V1 or V2 or 00 (/G) |
| 4 | 74HCT86 | offset inversion (subtraction, −1) |
| 4 | 74HCT283 | 16-bit adder, ripple carry |
| 2 + 2 | 74HCT377 + 74HCT541 | address latch + its bus_addr drivers (/OE per byte) |
| 2 + 2 | 74HCT377 + 74HCT541 | PC + its bus_data drivers (hi, lo) |
| 2 + 2 | 74HCT377 + 74HCT541 | SP + its bus_data drivers (hi, lo, P3) |

Every part is in Logisim's #TTL (chips.md). New part: **74HCT377** (CD74HCT377E): its DIP / SOIC references and its datasheet delays are to be checked before phase 2; the timings below use the 574's figures as placeholders.

### Timing estimate (max 25 °C = pass / max 85 °C = information, bus loading included)

| Path | Through | 25 °C | 85 °C | Limit | Margin 25 / 85 |
|---|---|---|---|---|---|
| control → OFFSET → sum → latch / PC / SP setup | CU 574 (33 / 41) → 157 S→Y (29 / 36) → 86 (32 / 40) → 283 A→C4 (48 / 60) → 2 × Cin→C4 (92 / 116) → Cin→S3 (53 / 66) → 377 setup (12 / 15) | 299 | 374 | 500 | 201 / 126 |
| **regs_a / regs_b → sum → setup** (Reg16, mode 4, IN/OUT) | operands valid (153 / 195, P2) → 153 I→Y (34 / 43) → 283 A→C4 → 2 × Cin→C4 → Cin→S3 (193 / 242) → setup (12 / 15) | 392 | 495 | 500 | **108 / 5** |
| IOD port on bus_addr → IVT → setup (vector step) | 153 I→Y (34 / 43) → 283 chain (193 / 242, counted as a full ripple) → setup (12 / 15) | 239 | 300 | 500 | the IOD must drive bus_addr by **261 / 200 ns** |
| latch → bus_addr valid | 377 CP→Q (33 / 41) → 541 A→Y (23 / 29) + load (~146 pF: +10 / +12) | 66 | 82 | — | memory gets the rest |
| PC / SP → bus_data (CAL, interrupt entry, RSP) | CU 574 (33 / 41) → 541 OE→Y (30 / 38) + data bus load (+37 / +49) | 100 | 128 | 335 / 325 (C10, P3) | 235 / 197 |

Bus loading from this section: `regs_a` and `regs_b` get 1 input per bit (the BASE 153), as P2 assumed. `bus_data` gets 4 three-state outputs (PC, SP) and **no inputs**: timing.py's estimate (2 in + 2 out each for PC and SP) drops by 4 inputs. `bus_addr` (low byte, worst): 3 outputs (latch, IOD, software vector) and about 5 inputs (memory, IOD, BASE, the software latch). Every path passes at 25 °C; the register path has 5 ns left at 85 °C (information only).

### Open questions (recommended answer first)

- **Q1. Pending software interrupt. Decided (Gianni, 2026-11-04): keep the port only.** A pending port latch in the control unit, 1 × 377 + 1 × 541, loaded while INT puts 00:V1 on bus_addr and driving the low byte back in the vector step (the address latch drives 00 on the high byte). The data is I when the interrupt is taken, so the vector step does not write I, and the pushed I is that same value. The 4-chip option (port and data, like the IOD) was dropped.
  - **Emulator changed to match** (cpu.rs: `pending_software_interrupt` is `Option<u8>`, the port; I is read on entry; the IDE's CPU view shows the port only). Before, cpu.rs kept (V1, I) from INT, and a handler saw the old I when I changed while the interrupt was pending.
  - Test: `pending_software_interrupt_takes_i_when_taken` (hbc2-core/tests/interrupts.rs): CLI, I := 11, INT 10, I := 22, STI → the handler gets I = 22, 22 is pushed, IRT restores 22.
  - Checks: the emulator test suite passes, except `hbc2-cc` `error_lines`, which fails the same way without this change (the uncommitted compiler/fcc work). All 11 `hbc2 vectors` modules are byte-identical before and after (none uses INT), so no vectors needed regenerating. The section tests (alu, flags, regs) were re-run on the existing circuits.
- **Q2. Interrupt entry order.** **Recommended:** push I and PC *before* raising INR, then take the port (bus_addr) and the data (→ I) in the vector step. Memory, registers and the wire protocol (INR one step, the IOD's reply one step) are the same as the manual's order; the manual's order would need the port and the data held for 3 steps (2–4 more chips).
- **Q3. PC and SP onto the data bus.** **Recommended:** separate drivers (4 × 541). Sharing 2 × 541 behind the BASE multiplexer saves 2 chips but CAL and interrupt entry take 2 more steps (the multiplexer can't show PC while it computes SP − 1); same reasoning as registers D1 (c).
- **Q4. RET / IRT / IVT bytes through the IR's V1 and V2.** **Recommended** (saves 4 × 157 and 4 data bus inputs, costs 1 step on RET, IRT and entry). Needs the IR to load V1 and V2 from the data bus in any step.
- **Q5. Invalid instruction.** **Recommended:** PC is incremented during the fetch; for `stops()` the control unit undoes it with PC − 1 (1 or 2 steps, then the CPU halts). Alternative: a fetch offset counter feeding OFFSET (more chips, every fetch).
- **Q6. 377 or counters for PC.** **Recommended:** 377 (PC + 1 goes through the adder with the latch, one step per byte). 4 × 74HCT161 would also count but adds 2 chips and still needs the adder for the other loads.

### For the control unit (to add to control-requirements.md with phase 2)

- C11: drive the section's controls from the clocked control register (BASE sel + 2 enables, OFFSET lo sel + 2 enables, inv, Cin, 3 load enables, 6 output enables). At most one driver on `bus_addr` (latch per byte, IOD, software vector). P4 applies (no load from `regs_a`/`regs_b` while `regs_we_n` is low).
- C12: the interrupt entry sequence above (Q2), including the vector step: latch outputs off, RAM and IOD not enabled by the CPU, I written from `bus_data` (hardware interrupts only).
- C13: the pending software interrupt port latch (Q1): 1 × 377 + 1 × 541, loaded by INT (00:V1 on bus_addr), cleared when taken, drives bus_addr low byte in the software vector step; I is not written there.
- C14: the IR loads V1 and V2 from `bus_data` in any step (Q4).
- C15: undo the fetch increments on an invalid instruction (Q5).
