# Register section: decisions (2026-11-04)

Module `regs`, in [circuits/registers/regs.circ](../circuits/registers/regs.circ), made by `designs/regs.py`. In scope: the 8 general registers, the write path, the operand latches, the A/B operand selection, the register index selection, the drivers to the data bus and to the address path. Out of scope (ports only): PC, SP, the address adder, the instruction register, memory, the control unit. What they must provide: [pc-sp-requirements.md](pc-sp-requirements.md), [control-requirements.md](control-requirements.md).

Source of truth: `hbc2-core/src/cpu.rs` and the mode lists of `isa.rs` (`INSTRUCTION_TABLE`). Indexes A=0 B=1 C=2 D=3 I=4 J=5 X=6 Y=7. R1 = instruction bits 21..19, R2 = 18..16, R3 = the low 3 bits of V1 (`Instruction::r3`, mode 4 only).

## 1. Every use of a general register (from cpu.rs)

"mem" = a RAM byte, read or written through the data bus. Address sources in brackets belong to the address path, not to this section.

| Instruction | Mode | Reads | Writes | Data comes from → goes to |
|---|---|---|---|---|
| ADD ADC SUB SBB AND OR XOR | Reg | R1 (A), R2 (B) | R1 | R1 op R2 → R1 |
| same | RegImm8 | R1 (A) | R1 | R1 op V1 → R1 |
| same | RegRam | R1 (A) | R1 | R1 op mem[Vx] → R1 |
| same | mode 4 | R1:R2 (address), R3 (A) | R3 | R3 op mem[R1:R2 + V2] → R3 |
| same | StackOffset | R1 (A) | R1 | R1 op mem[SP + V1] → R1 |
| CMP | Reg, RegImm8, mode 4, StackOffset (no RegRam) | as above | none (flags S E F) | |
| INC DEC | Reg | R1 (A) | R1 | R1 ± 1 → R1 |
| INC DEC | Imm16 | none | none | mem[Vx] ± 1 → mem[Vx] |
| INC DEC | mode 4 | R1:R2 (address) | none | mem[R1:R2 + V2] ± 1 → same address (V1 unused) |
| NOT | Reg | R1 (A) | R1 | not R1 → R1 |
| NOT | Imm16 | none | none | not mem[Vx] → mem[Vx] |
| SHL ASR SHR | Reg | R1 (A) | R1 | shifted R1 → R1 |
| MOV | Reg | R2 | R1 | R2 → R1 |
| MOV | RegImm8 | none | R1 | V1 → R1 |
| LOD | RegRam / StackOffset | none | R1 | mem[Vx] / mem[SP + V1] → R1 |
| LOD | mode 4 | R1:R2 (address) | R3 | mem[R1:R2 + V2] → R3 |
| STR | RegRam / StackOffset | R1 | none | R1 → mem[Vx] / mem[SP + V1] |
| STR | mode 4 | R1:R2 (address), R3 | none | R3 → mem[R1:R2 + V2]: **three registers** |
| PSH | Reg | R1 | none | R1 → mem[SP − 1] |
| POP | Reg | none | R1 | mem[SP] → R1 |
| IN | Reg | R2 (port) | R1 | port R2 → R1; address bus = R2 (high byte 00) |
| OUT | Reg | R1 (port), R2 (data) | none | R2 → port R1; address bus = R1 (high byte 00) |
| JMP JMC JME JMN JMS JMZ JMF, CAL | Reg16 | R1:R2 | none | R1:R2 → PC |
| WSP | Reg16 | R1:R2 | none | R1:R2 → SP |
| RSP | Reg16 | none | R1, then R2 | SP high → R1, SP low → R2 |
| INT | Imm8 | I (fixed index 4) | none | I → data bus, V1 → address bus |
| interrupt entry | | I (push) | I | I → mem[SP − 1]; data bus (from the IOD) → I |
| IRT | None | none | I | mem (popped) → I |
| reset | | none | all 8 | 00 → every register |

No other instruction touches a general register (NOP, flag set/clear, HLT, PSF, POF, RET, ASP, SSP, the Imm16 jumps, WSP Imm16).

What follows from the table:
- **Read port A** needs R1, R2 (OUT data, MOV), R3 and a fixed index from the control unit (I, the reset sweep). **Read port B** needs R2, and R1 for OUT's port number.
- **Writes** go to R1, R2 (RSP), R3, I (interrupt, IRT) or any index (reset). Every write happens in a step where port A is free, so **the write index is port A's index** (D3, below).
- **Three registers at once** (STR mode 4): two read ports can't do it, so the address path must latch the address (R1:R2 + V2) in one step and the data (R3) goes out in the next. The same address latch breaks two loops: LOD R3 with R3 = R1 or R2, and IN R1, R2 with R1 = R2 (the written register is also the address). pc-sp-requirements.md P1.
- **A memory operand on A**: INC/DEC/NOT on memory (Imm16, mode 4). On B: every other ALU op with a memory operand, and V1.

## 2. Choices

### D1. 74HCT670 register file, two copies (the proposed design, kept)

Options, with chip counts for the storage and read ports only:
- (a) **8 × 74HCT670**, two copies of 4 chips (words 0–3 and 4–7, two nibbles each); every write goes to both copies, each copy is one read port. **Chosen.**
- (b) one 74HCT574 per register + one 74HCT541 each for the second port: 16 chips plus 3 decoders, and an edge-triggered register can only load on a chosen step with a gated clock or 2 × 74HCT157 of feedback each. More chips and it breaks the no-gated-clock rule.
- (c) one copy (4 × 670) + an 8-bit holding latch: 5 chips (−3), but every instruction that needs two registers (ALU Reg mode, mode 4 addresses, the Reg16 modes) takes one more step. Not worth 3 chips.

- (d) 8 × 74HCT377 (edge-triggered, load enable) + 16 × 74HCT541 (one per read port per register) + 3 × 74HCT138 (read A, read B, write), compared on paper on 2026-11-04 because the 670 costs about $15 (120–200 in stock): 36 chips instead of 18, 692 pins instead of 306, about $37 instead of $131 for the section. Writes on the clk edge, so no write strobe and write data needed by about 485 ns instead of 325; but 8 + 1 outputs on bus A (362 pF) make the read path 208 ns instead of 195 (end to end 533 at max 85 °C). **Gianni kept the 670 version** (2026-11-04).

Nothing found beats (a) on both chips and timing.

### D2. Free-running latches, no enables

The result latch and the two operand latches are 74HCT574s clocked by `clk` on **every** edge: the result latch always holds what the ALU computed in the step before, the operand latches what was on the data bus in the step before. No gated clock and no load enable; the control unit just uses them in the next step. This fits every sequence in the table: a memory read is always followed directly by its ALU step, and V1 is fetched in the step right before the ALU step of a RegImm8 instruction, so **V1 reaches B through the operand latch** (it is captured while it is fetched). That saves the IR → B buffer (−1 chip). The cost: an operand or a result is held for one step only.

### D3. One index for port A and for the write

The write address of all 8 chips and the read address of copy A come from the same multiplexer (`regs_a_sel`). Every write in the table happens in a step where port A is free or reads the register being written, so a separate write multiplexer (2 × 74HCT153) isn't needed. While /WE is low, port A shows the new data (the 670 is transparent): harmless, because nothing stores port A's value in a write step (D6).

### D4. Two operand latches

INC/DEC/NOT on memory take the operand on A, every other memory operand goes on B (and A − 1 isn't 1 − A, so DEC can't swap them). One 74HCT574 can drive only one bus, so there are two, both loaded from the data bus on every edge: `regs_a_lat_oe_n` puts one on A, `regs_b_lat_oe_n` the other on B. A 574 + 541 pair would cost the same.

### D5. Internal tri-state buses A and B, bank enables from the index multiplexers

- **Bus A** (`regs_a`, to `alu_a` and to the address path high byte): copy A (4 × 670, /RE) or the A operand latch.
- **Bus B** (`regs_b`, to `alu_b` and to the address path low byte): copy B (/RE), the B operand latch, or the constant buffer.
- Each copy's two banks are selected by /RE: words 0–3 when index bit 2 = 0, words 4–7 when it is 1. The obvious way, a 74HCT139 decoder after the index multiplexer, puts the operands at 41 + 43 + 43 + 48 = **175 ns**, which leaves exactly 0 ns for the ALU-to-flags path (325 ns, without bus loading). So the index multiplexers make the bank enables directly: one 74HCT153 half carries bit 2 (= /RE of words 0–3), another carries **not** bit 2 (= /RE of words 4–7). The inverted bit 2 of R1, R2 and R3 comes **from the IR** (`regs_r1_2_n` ..., control-requirements.md C8; Gianni, 2026-11-04): the IR makes it before its latch, during the fetch, where there is time. The first version had a 74HCT240 here, 28 ns on the read path and one chip.
- **Turning a port off**: port B's fourth source (`regs_b_sel` = 11) has both bank enables tied high. Port A's fourth source is the control unit's index, which comes with its own two bank enables (`regs_cu_bank_n`); 11 there turns port A off. With port A off no write can happen (/WE comes from the same enables), which is safe.
- The control unit must enable **exactly one source per bus** in every step (control-requirements.md C5). Like the ALU's result bus (alu-decisions.md D7), two drivers can overlap for a few ns when the source changes (670 output disable is 44 ns max, a 574 enables in 38 ns).

### D6. Writes come from the data bus only

The 670 D inputs are `regs_bus_in`, i.e. the data bus. Each write source reaches it:
- ALU result: the result latch drives the data bus (`regs_res_oe_n`). The same path writes INC/DEC/NOT results to memory.
- LOD, POP, IN, interrupt data, IRT: memory or the IOD drives the bus.
- MOV R1, imm: written straight from the bus while V1 is fetched.
- MOV R1, R2: through the ALU, as AND R2, R2 (A = B = R2) into the result latch, then written to R1 (2 steps, no extra chip; the flags are not stored). A direct copy R2 → data bus → R1 would also work in one step, but if R1 = R2 it closes a loop through the transparent 670, so it isn't used.
- RSP: the SP section drives SP high, then SP low onto the bus (pc-sp-requirements.md P3).
- Reset: the ALU makes the zeros, see D8.

A separate write bus (result latch + data bus multiplexer, 2–3 chips) would let a write overlap the next fetch. Not worth it at this stage; on the review list.

**Why the transparent write can't loop.** While /WE is low, the written word follows D, and its outputs follow too (63 ns). A loop needs a path from a 670 output back to D inside the step. The paths into D (`regs_bus_in`) are:
- the result latch: edge-triggered, its Q is constant during the step, whatever the ALU does with port A's changing value;
- memory and the IOD: their address comes from the address latch (edge-triggered, pc-sp-requirements.md P1), never straight from bus A/B in a write step;
- SP: not a register output;
- this section's own bus-A driver (`regs_a_bus_oe_n`): the only combinational path from a 670 to the data bus. The control unit must keep it off whenever `regs_we_n` is low (control-requirements.md C5).

So every path from a 670 output to D goes through a flip-flop clocked by `clk`, which doesn't change during /WE.

### D7. The write pulse: a second clock phase

The 670 writes while /WE is low (transparent: the word follows D) and keeps what D holds when /WE rises. The write address must be stable for the whole low pulse (datasheet note 1; setup 23 ns before /WE falls, hold 5 ns after it rises); D must be set up 15 ns before /WE **rises** and held 5 ns after (chips.md "Delays"; the first version of this file asked for D 15 ns before /WE falls, which was stricter than the datasheet). Writing during the clock's low half doesn't work: /WE would rise at the next rising edge, the same moment the result latch and the index change, a race. So /WE needs an edge of its own **inside** the step.

**Clock generator** (control-requirements.md C6; Gianni, 2026-11-04: one 4 MHz oscillator and a 74HCT74 divider, so both phases come from one source):

    osc (4 MHz) -> FF1 CLK, D1 = /Q1        clk = Q1 (2 MHz), clk_n = /Q1
    osc -> NAND as inverter -> FF2 CLK, D2 = clk     Q2 = clk, half an osc period later
    clk_wr_n = NAND3(clk_n, osc, Q2)        (74HCT10: one gate here, one as the inverter)

(**Kept, 2026-11-05:** a 74HCT32 + 74HCT14 replacement for the NRND 74HCT10 was tried and reverted by Gianni: its pulse ended up to 56 ns later, 3.5 ns before the next edge at 55 % duty. The 74HCT10 stays, with spares: cu-decisions.md sections 12 and 14, chips.md.)

`clk_wr_n` is low while clk is low, osc is high and Q2 is high: from the clk fall (an osc rise, 250 ns) to the next osc fall (375 ns). Q2 is there for the osc rise at 500 ns: osc rises a flip-flop delay before clk_n falls, so NAND(clk_n, osc) alone would glitch low; Q2 is already low then (it falls by 440 ns: the inverter 30 + FF2 35 after the osc fall). At every transition only one input of the NAND moves while the others hold it, so the strobe is glitch-free. Parts: the oscillator, a 74HCT74 and a 74HCT10 (one gate spare).

In the section (74HCT32, U4):

    en_lo_n = regs_we_n OR /RE_A(0-3)     /WE words 0-3 = clk_wr_n OR en_lo_n
    en_hi_n = regs_we_n OR /RE_A(4-7)     /WE words 4-7 = clk_wr_n OR en_hi_n

`clk_wr_n` goes through one gate. (It went through two, `regs_wr_n` = clk_wr_n OR regs_we_n first: with the generator's own delays the end of the pulse would then come 30 ns later, 470 + hold, too close to the edge.) The enables only depend on control signals, and are ready at 117 ns.

It respects the earlier rules:
- **No gated clock**: every flip-flop in the CPU still takes `clk` (rising edge) and nothing else. The 670 is a level-sensitive latch, and /WE is its write strobe, like the RAM's /WE (the memory needs the same kind of strobe for its writes, so the clock generator provides it once).
- **F1 (the ALU gets a full step with stable inputs)**: no register is written in an ALU step, so the operands don't move between 195 ns and the next edge.
- The control signals (`regs_we_n`, the indexes) change only at the clock edge, so they are stable for the whole strobe.

One step, at 85 °C max (the worst case drawn; the pass criterion is max 25 °C, section 6) (t = 0 is the rising edge of `clk`; minimum delays taken as 0, so clk can rise up to 35 ns after its osc edge and fall 35 ns early or late):

```
t (ns)        0   41      117 132  195   215  250      340 345 375    440  500
osc           ‾‾‾‾‾‾‾‾‾|_______________________|‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾|______________|‾‾
clk           ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾|____________________________|‾‾‾
IR/CU outputs XXXXX|====================== stable ========================|XXX
write index   XXXXXXXXXXXXXXXX|========= stable, set up (132) ==============|XXX
read port Q   XXXXXXXXXXXXXXXXXXXXXXX|== valid (195) ===========================
data bus (D)  XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX|== valid by 325 =|XXX
clk_wr_n      ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾|_______________|‾‾‾‾‾‾‾‾‾‾‾‾‾
/WE (bank)    ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾|_______________|‾‾‾‾‾‾‾‾‾‾‾
                                        falls 215..345   rises 340..435
ALU step      operands valid 195 → ALU + flags 325 → 520: over (section 6)
```

Figures as max 25 °C (the pass criterion) / max 85 °C (information):
- Write address: set up at 104 / 132 ns, before the earliest /WE fall (222 / 215 ns): **118 / 83 ns of margin**. The bank write enables are ready at 93 / 117 ns.
- Data: `bus_data` must be valid by **335 / 325 ns** (the earliest /WE rise, 347 / 340, minus the setup 12 / 15) and held until the clock edge (C10). The result latch drives it at 100 / 128 ns (with the bus load).
- Hold: /WE rises by 423 / 435 ns; D and the address change at 500 ns at the earliest: **72 / 60 ns of margin** (5 ns needed).
- Pulse: at least **49 / 30 ns** (20 / 25 needed): the 125 ns from osc rise to osc fall, minus the delays that can only make it start later (FF1, the NAND, the OR). Pessimistic (no minimum delays). Fallback if the hardware shows write errors: 74ACT74 + 74ACT10 in the clock generator, about 75 ns at 85 °C (chips.md "Fallbacks").
- The 670's "latch time" (31 ns, read address held after /WE rises) is met: the index holds until the edge.

### D8. Reset: zeros from the ALU

The 670 has no reset input. The control unit runs a reset sweep (control-requirements.md C7): 9 steps with `alu_op` = AND, B = the constant 00, the result latch on the data bus. Step 0 puts 00 in the result latch; steps 1 to 8 write it to index 0..7 (`regs_a_sel` = 11, `regs_cu_idx`, `regs_cu_bank_n`). AND with 00 gives 00 whatever port A shows, so the result latch stays at 00 although port A changes during each write. Nothing else is needed: the constant buffer takes `regs_const_0` as its bit 0 instead of a tied 1, which gives 00 or 01 on B.

### D9. Port B and the address path

`regs_a` and `regs_b` go to the ALU and to the address path. R1:R2 as an address or a 16-bit value (Reg16, mode 4) = `regs_a`:`regs_b` with A = R1, B = R2. IN's port is R2 (B = R2), OUT's port is R1 (B = R1), the data R2 goes out from port A. The address path puts 00 in the high byte for IN and OUT, as cpu.rs does (pc-sp-requirements.md P1).

### D10. Data bus drivers

- `regs_a_bus_oe_n`: a 74HCT541 puts bus A on the data bus: STR (R1 or R3), PSH, OUT (R2), INT and interrupt entry (I).
- `regs_res_oe_n`: the result latch on the data bus: register writes of ALU results, INC/DEC/NOT to memory.

Without the 541 a store could go through the ALU (ADD R, 00 into the result latch): −1 chip, +1 step for every STR/PSH/OUT. On the review list.

### D11. Names and files

- Module `regs` (not `regfile`): it holds more than the register file. Signals `regs_*`. One module, one file, no submodules: everything shares the same buses, so splitting would only add tunnels.
- **The control unit's requirements stay in `control-requirements.md`**, the file that already holds C1–C4, instead of a new `cu-requirements.md`: one file per section. The IR and memory requirements go there too (the IR belongs to the control unit, and the control unit schedules memory).
- `pc-sp-requirements.md` is new: PC, SP, the address adder and the address latch.

## 3. Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `clk` | 1 | top | rising edge: the three 574 latches load (always) |
| `clk_wr_n` | 1 | top | write phase, low from the clk fall (250 ns) to the next 4 MHz fall (375 ns) of each step (clock generator, D7) |
| `regs_we_n` | 1 | top | 0: write `regs_bus_in` into the register at port A's index during `clk_wr_n` |
| `regs_a_sel` | 2 | top | port A / write index: 00 R1, 01 R2, 10 R3, 11 the control unit's index |
| `regs_cu_idx` | 2 | top | control unit's index, bits 1..0 |
| `regs_cu_bank_n` | 2 | top | control unit's index, bank: 10 = words 0–3, 01 = words 4–7 (I = 4: idx 00, bank 01), **11 = port A off** (no write possible), 00 forbidden |
| `regs_b_sel` | 2 | top | port B index: 00 R2, 01 R1, 10 R3, **11 port B off** |
| `regs_a_lat_oe_n` | 1 | top | 0: A operand latch onto bus A (port A must be off) |
| `regs_b_lat_oe_n` | 1 | top | 0: B operand latch onto bus B (port B off) |
| `regs_const_oe_n` | 1 | top | 0: the constant 0000000c onto bus B (port B off) |
| `regs_const_0` | 1 | top | c: 1 for INC/DEC (01), 0 for the reset zeros (00) |
| `regs_a_bus_oe_n` | 1 | top | 0: bus A onto the data bus (never with `regs_we_n` low) |
| `regs_res_oe_n` | 1 | top | 0: the result latch onto the data bus |
| `regs_r1`, `regs_r2`, `regs_r3` | 3 each | left | instruction fields (IR) |
| `regs_r1_2_n`, `regs_r2_2_n`, `regs_r3_2_n` | 1 each | left | not bit 2 of each field, from the IR (D5) |
| `regs_alu_r` | 8 | left | ALU result (`alu_r`) |
| `regs_bus_in` | 8 | left | data bus, in: write data and operand latches |
| `regs_a` | 8 | right | bus A: `alu_a`, and the address path high byte |
| `regs_b` | 8 | right | bus B: `alu_b`, and the address path low byte |
| `regs_bus_out` | 8 | right | data bus, out, tri-state (result latch or bus A) |

In `main`, `regs_bus_in` and `regs_bus_out` both connect to `bus_data` (same split as `flags_bus_in` / `flags_bus_out`).

## 4. Design

| Chip | Part | Does |
|---|---|---|
| U2, U7 | 74HCT670 | copy A, words 0–3: bits 3..0, 7..4 |
| U12, U16 | 74HCT670 | copy A, words 4–7: bits 3..0, 7..4 |
| U3, U8 | 74HCT670 | copy B, words 0–3 (read = port B; write address = port A's index) |
| U13, U17 | 74HCT670 | copy B, words 4–7 |
| U1 | 74HCT153 | port A / write index, bits 1..0: R1 R2 R3 CU |
| U6 | 74HCT153 | port A bank enables: bit 2 → /RE words 0–3, not bit 2 (from the IR) → /RE words 4–7 (CU: `regs_cu_bank_n`) |
| U11 | 74HCT153 | port B index, bits 1..0: R2 R1 R3 0 |
| U15 | 74HCT153 | port B bank enables (both high on 11 = off) |
| U4 | 74HCT32 | per bank: enable = regs_we_n OR /RE of that bank on port A; /WE = clk_wr_n OR enable (D7) |
| U9 | 74HCT541 | the constant 0000000c onto bus B |
| U14 | 74HCT541 | bus A onto the data bus |
| U5 | 74HCT574 (model) | result latch: alu_r → data bus |
| U10 | 74HCT574 (model) | operand latch: data bus → bus A |
| U18 | 74HCT574 (model) | operand latch: data bus → bus B |

Chips are drawn facing south (the datasheet's top view) and wired by tunnels named after the signals, like the flag register. Bill of materials: [bom-regs.csv](bom-regs.csv).

**18 chips**: 74HCT670 × 8, 74HCT153 × 4, 74HCT574 × 3, 74HCT541 × 2, 74HCT32 × 1. (19 with the 74HCT240 of the first version, D5.)

### D12. Chip models are named `ttl<part>` (Logisim finding)

The three 574 latches read U (undefined) in Logisim although the same model passed its own bench. Cause, found by elimination: **Logisim-evolution 5.0 doesn't simulate a library circuit whose name starts with a digit** when it is instantiated from another file (a plain wire from A to Y named `74574` or `74hct574` fails, `x74574` passes; the same circuit in the same file works). The model circuit is now `ttl74574`; the file stays `circuits/chips/74574.circ`, its bench stays `test_74574`, and the part number is still 74574 everywhere else (bom.py, chips.md). `netlist.chip_model` and conventions.py know the `ttl` prefix. Any future model must use it too (conventions.md "Components").

## 5. Tests

Both are sequences (Logisim's `<set>`/`<seq>`), generated by `hbc2 vectors` (`hbc2-cli/src/vectors/regs.rs`) and run by `python designs/regs.py --test`.

**Bus contention fails the tests** (Gianni, 2026-11-04; conventions.md "Tests"). Logisim shows two drivers that disagree as E, and logitest reports any E as `BUS CONTENTION`. An E passes where the vector says don't care, so the three outputs (`regs_a`, `regs_b`, `regs_bus_out`, all tri-state buses) are always checked: a value, `<FLOAT>` when nothing may drive them, or "unknown" when the model doesn't know the value (a register read before anything was written to it since power-up: 8 rows per sequence, while the registers are loaded). `hbc2 vectors` refuses a don't care on a bus (`SeqModule::buses`). Port B is off while the registers are loaded, so bus B is checked as `<FLOAT>` there. Logisim shows no E when the two drivers happen to agree, so the data is random.

| Vectors | What | Result |
|---|---|---|
| `hbc2 vectors regs` | against a model of the section: the reset sweep (D8); every pair of indexes on both ports from every field source (R1/R2, R2/R1, R3/R3, CU/R3), twice; a write to every index from each A source (R1, R2, R3, CU) while port B reads every index, the written one included (it must follow D while /WE is low); every A source × every B source × every data bus driver; writes with port A off or on the latch (must not write); 40 random sequences of 200 valid steps | **30,756 / 30,756** |
| `hbc2 vectors regs_cpu` | the real CPU runs one instruction from random registers and random memory; the rows load the 8 registers, play the control unit's steps (below), then read the 8 registers back. Every operand the section presents, every byte it puts on the data bus and the final registers are asserted against the CPU while the vectors are generated, and checked in Logisim. MOV (Reg, Imm), LOD and STR (RegRam, StackOffset, mode 4), PSH, POP, IN, OUT, RSP, ADD/SUB/XOR/AND in Reg, RegImm8, RegRam, StackOffset and mode 4, INC Reg / Imm16 / mode 4, NOT Imm16. Every (R1, R2) pair; mode 4 LOD/STR/ADD with every R3 too (512 combinations each), so every aliasing case (R3 = R1, R1 = R2 ...) is covered | **148,864 / 148,864** |

The control unit's steps used by `regs_cpu` (one line per clock step; "write" = `regs_we_n` low):

| Instruction | Steps |
|---|---|
| MOV R1, R2 | A = R2, B = R2, alu_op AND → result latch; A = R1, result latch → data bus, write |
| MOV R1, imm | while V1 is on the data bus: A = R1, write |
| LOD R1 (RegRam, Stack), POP R1 | memory on the data bus: A = R1, write |
| LOD R3, mode 4 | A = R1, B = R2 → address latch; memory on the bus: A = R3, write |
| STR R1 (RegRam, Stack), PSH R1 | A = R1, bus A → data bus |
| STR R3, mode 4 | A = R1, B = R2 → address latch; A = R3, bus A → data bus |
| IN R1, R2 | B = R2 → address latch (port); IOD on the bus: A = R1, write |
| OUT R1, R2 | B = R1 → address latch (port); A = R2, bus A → data bus |
| RSP R1, R2 | SP high on the bus: A = R1, write; SP low on the bus: A = R2, write |
| op R1, R2 | A = R1, B = R2 → result latch (flags stored); A = R1, result latch → data bus, write |
| op R1, imm / $(Vx) / (SP + V1) | V1 or memory on the bus (→ operand latches); A = R1, B = operand latch; write R1 |
| op R3, mode 4 | A = R1, B = R2 → address latch; memory on the bus; A = R3, B = operand latch; write R3 |
| INC R1 | A = R1, B = constant 01; write R1 |
| INC / NOT on memory | (mode 4: address step); memory on the bus; A = operand latch, B = 01 (INC); port A off, result latch → data bus (memory write) |

## 6. Timing

`python tools/timing.py --section regs`. **Pass criterion: max 25 °C**, bus loading included (Gianni, 2026-11-04); max 85 °C is shown for information. Each path with a limit shows its margin in both columns. The IR fields and the control signals come from registers clocked by `clk` (74HCT574, 41 ns), like the flag register's analysis. **Bus loading is counted** (added 2026-11-04, Gianni): a stage that drives a bus gets the extra delay of its load beyond the datasheets' 50 pF.

| Path | Through (per stage, max 25 / 85 °C, ns) | Typ | Max 25 °C | Max 85 °C | Limit (25 / 85 °C) | Margin 25 °C (pass) | Margin 85 °C (info) |
|---|---|---|---|---|---|---|---|
| index → word read (bits 1..0) | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 read address → Q (40 + 27 / 50 + 36 load) | 78 | 153 | 195 | — | — | — |
| bank enable (bit 2, or not bit 2 from the IR) | 574 CP → Q (33 / 41) → 153 I → Y (34 + 2 / 43 + 3 load) → 670 /RE → Q (38 + 27 / 48 + 36 load) | 66 | 134 | 171 | — | — | — |
| operand latch onto bus A / B | 574 CP → Q (33 / 41) → 574 OE → Q (30 + 18 / 38 + 24 load) | 40 | 81 | 103 | — | — | — |
| constant 00 / 01 onto bus B | 574 CP → Q (33 / 41) → 541 OE → Y (30 + 13 / 38 + 17 load) | 46 | 76 | 96 | — | — | — |
| **operands valid** on regs_a / regs_b (alu_a, alu_b, address path) | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 read address → Q (40 + 27 / 50 + 36 load) | 78 | 153 | 195 | — | — | — |
| **end to end**: register read → ALU (alu_z) → flag register setup | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 read address → Q (40 + 27 / 50 + 36 load) → [alu_z] (205 / 257) → 153 I → Y (34 / 43) → 175 setup D → CLK (20 / 25) | 207 | 412 | 520 | ≤ 500 | 88 | **-20** |
| register read → ALU (alu_r) → result latch setup | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 read address → Q (40 + 27 / 50 + 36 load) → [alu_r, arithmetic (bit 7)] (154 / 193) → 574 setup D → CP (12 / 15) | 165 | 319 | 403 | ≤ 500 | 181 | 97 |
| register → data bus (STR, PSH, OUT, INT) | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 read address → Q (40 + 27 / 50 + 36 load) → 541 A → Y (23 + 37 / 29 + 49 load) | 117 | 213 | 273 | ≤ 500 | 287 | 227 |
| result latch → data bus (enable) | 574 CP → Q (33 / 41) → 574 OE → Q (30 + 37 / 38 + 49 load) | 53 | 100 | 128 | ≤ 335 / 325 (data on bus_data valid by) | 235 | 197 |
| write address + setup, before /WE falls | 574 CP → Q (33 / 41) → 153 S → Y (34 + 19 / 43 + 25 load) → 670 setup WA → /WE (18 / 23) | 60 | 104 | 132 | ≤ 222 / 215 (/WE falls, earliest) | 118 | 83 |
| bank write enable, stable before clk_wr_n falls | 574 CP → Q (33 / 41) → 153 I → Y (34 + 2 / 43 + 3 load) → 32 A/B → Y (24 / 30) | 46 | 93 | 117 | ≤ 222 / 215 (/WE falls, earliest) | 129 | 98 |
| /WE falls, earliest | osc rises (250) → − 74 CLK → Q (28 / 35) | 230 | 222 | 215 | — | — | — |
| /WE falls, latest | osc rises (250) → 74 CLK → Q (28 / 35) → 10 A/B/C → Y (24 / 30) → 32 A/B → Y (24 / 30) | 294 | 326 | 345 | — | — | — |
| /WE rises, earliest (D setup is to this edge) | osc falls (375) → − 74 CLK → Q (28 / 35) | 355 | 347 | 340 | — | — | — |
| **data on bus_data valid by** (the earliest rise − setup) | [/WE rises, earliest (D setup is to this edge)] (347 / 340) → − 670 setup D → /WE (12 / 15) | 343 | 335 | 325 | — | — | — |
| /WE pulse, shortest | osc rise to osc fall (125) → − 74 CLK → Q (28 / 35) → − 10 A/B/C → Y (24 / 30) → − 32 A/B → Y (24 / 30) | 81 | 49 | 30 | ≥ 20 / 25 (670 /WE pulse width) | 29 | 5 |
| /WE rises, latest, + hold | osc falls (375) → 10 A/B/C → Y (24 / 30) → 32 A/B → Y (24 / 30) → 670 hold D, WA after /WE (5 / 5) | 404 | 428 | 440 | ≤ 500 | 72 | 60 |

| Bus (worst bit) | Inputs | 3-state outputs | Load (pF) | Extra delay, bus driver (max 25 / 85 °C, ns) | Extra delay, standard output (max 25 / 85 °C, ns) | Counted (worst bit of each) |
|---|---|---|---|---|---|---|
| bus_a | 12 | 3 | 230 | 18 / 24 | 27 / 36 | regs: regs_a: 1 in, 3 out; alu: alu_a: 10 in, 0 out; address path: adder input, R1 high byte (not built): 1 in, 0 out |
| bus_b | 6 | 4 | 180 | 13 / 17 | 20 / 26 | regs: regs_b: 0 in, 4 out; alu: alu_b: 5 in, 0 out; address path: adder input, R2 low byte (not built): 1 in, 0 out |
| bus_data | 15 | 10 | 420 | 37 / 49 | 56 / 74 | regs: regs_bus_in: 6 in, 0 out; regs: regs_bus_out: 0 in, 2 out; flags: flags_bus_in: 1 in, 0 out; flags: flags_bus_out: 0 in, 1 out; IR: opcode, byte 1, V1, V2 latches (not built): 4 in, 0 out; PC: load hi / lo, drive hi / lo for CAL (not built): 2 in, 2 out; SP: load hi / lo, drive hi / lo for RSP (not built): 2 in, 2 out; RAM, ROM, I/O port data (not built): 0 in, 3 out |
| a_idx | 12 | 1 | 171 | 13 / 16 | 19 / 25 | regs: regs_a_idx: 12 in, 1 out |
| b_idx | 4 | 1 | 75 | 3 / 4 | 4 / 5 | regs: regs_b_idx: 4 in, 1 out |
| a_re_lo | 3 | 1 | 63 | 2 / 2 | 2 / 3 | regs: regs_a_re_lo_n: 3 in, 1 out |
| a_re_hi | 3 | 1 | 63 | 2 / 2 | 2 / 3 | regs: regs_a_re_hi_n: 3 in, 1 out |
| b_re_lo | 2 | 1 | 51 | 1 / 1 | 1 / 1 | regs: regs_b_re_lo_n: 2 in, 1 out |
| b_re_hi | 2 | 1 | 51 | 1 / 1 | 1 / 1 | regs: regs_b_re_hi_n: 2 in, 1 out |

How the load is estimated (timing.py "bus loading"):
- **Pins**: 10 pF per input (CI max), 20 pF per 3-state output, on or off (CO, CD74HC670 datasheet). They are counted on the circuits (netlist.py `flatten`, which now counts chip models as chips): bus A = 3 outputs and 1 input in the section + 10 ALU inputs on its worst bit; bus B = 4 outputs + 5 ALU inputs. The sections not built yet are estimates, listed in the table (the address path; on the data bus: IR, PC, SP, RAM, ROM, I/O).
- **Wiring**: 2 pF per pin + a 20 pF backbone for the three buses, 5 pF for the local nets.
- **Delay**: SN74HCT541 (a 6 mA bus driver) is 29 ns at 50 pF and 42 ns at 150 pF, max 85 °C: **0.13 ns per pF**. Standard 4 mA outputs (670, 153) have no 150 pF figures; scaled by drive, **0.20 ns per pF**.
- The local nets matter as much as the buses: each port A index bit goes to 4 read addresses and 8 write addresses (171 pF, +25 ns on the 153).

Results (max 25 °C / max 85 °C):
- **Every limit is met at max 25 °C.** The tightest is the end to end path: **412 ns, 88 ns of margin**. Register read (153) + the ALU's slowest flag (alu_z, 205) + the flag register's input multiplexer and setup (54).
- At max 85 °C the end to end path is 520 ns, 20 ns over the step (information only). Without the load it would be 459 ns: the loads add 61 ns there (25 on the index lines, 36 on bus A).
- The result latch: 319 / 403 ns. Register → data bus: 213 / 273 ns (the data bus is the heaviest net, about 420 pF).
- Write: see D7.

**Fallback, not fitted (Gianni, 2026-11-04: stay all-HCT): 74ACT153 instead of 74HCT153**, only if the hardware shows timing errors (chips.md "Fallbacks"), for the 4 index multiplexers here and the 4 input multiplexers of the flag register. Same pinout and logic (no circuit change, only the BOM), TTL-level inputs like HCT. CD74ACT153, max 85 °C: select → Y 20 ns, data → Y 16.4 ns (against 43); 24 mA outputs, so the load costs about 0.035 ns per pF.

| Option | Operands valid | End to end | Margin |
|---|---|---|---|
| all HCT (now) | 195 | 520 | −20 |
| 74ACT153 × 4 in the register section | 152 | 477 | 23 |
| + 74ACT153 × 4 in the flag register (D2, the ALU input) | 152 | 451 | **49** |

(Computed with timing.py and the CD74ACT153 figures.) Caveat: AC parts switch fast (edges of about 3 ns), so they need good decoupling and short lines. Here they only drive local nets on the same board (the 670 addresses and enables, the 175 inputs), none of the buses. Other ways, each worse: a 541 between bus A and the ALU (+1 chip, the 670 drives less but the buffer adds 29 + its own load: no gain); a separate write-address multiplexer so the index lines drive half the load (+1 chip, about −20 ns); a slower clock.

## 7. Review

Gianni's answers (2026-11-04) to the first list:

| # | Question | Decision | Done |
|---|---|---|---|
| 1 | 15 ns of margin | IR supplies bit 2 inverted; count bus loading; report, and propose the next fix if under 30 ns | U1 (240) removed, 18 chips; timing.py counts loads; **−20 ns** with loads at max 85 °C, next fix proposed (section 6) |
| 2 | `clk_wr_n` | OK for 4 MHz; clk and clk_wr_n from one 74HCT74 divider | D7, control-requirements.md C6 |
| 3 | `ttl` prefix | OK | — |
| 4 | Address latch | hard requirement for PC/SP | pc-sp-requirements.md P1 |
| 5 | Write bus, U14 | no write bus; keep U14 (stores and pushes are frequent) | — |
| 6 | One driver per bus | left to the control unit, but any E on a bus fails the tests, in every instruction-level test from now on | logitest, `hbc2 vectors` (section 5), conventions.md "Tests" |

Second list (Gianni, 2026-11-04):

| # | Question | Decision | Done |
|---|---|---|---|
| 1 | End to end 20 ns over the step at 85 °C | keep the 670 version (D1 (d)); stay all-HCT; the pass criterion is max 25 °C with bus loading, 85 °C informative | timing.py: limits, margins at 25 and 85 °C, exit 1 on a 25 °C failure; 88 ns of margin at 25 °C |
| 2 | /WE pulse 30 ns at 85 °C | 74ACT74 + 74ACT10 (clock generator) and 74ACT153 (registers, flags) recorded as drop-in fallbacks | chips.md "Fallbacks"; pulse 49 ns at 25 °C (20 needed) |

Open:

3. **Data setup to /WE rising** (D7): I now read the 670's data setup against the rising edge of /WE (the latch closes then; chips.md had it so), not the falling one. Data for a register write must be valid by 325 ns instead of 235 (C10, P3). **Confirmed by Gianni (2026-11-04).**
4. **The load estimates for the sections not built** (section 6): 15 pins on the data bus. timing.py lists them, to be replaced by counts when those sections exist.
5. Emulator: no change, no bug found. Every step list in section 5 is asserted against the CPU while the vectors are generated.
