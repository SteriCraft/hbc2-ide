# ALU: report (unattended run, night of 2026-11-04)

The `alu` section now covers every ALU instruction of the HBC-2: ADD ADC SUB SBB INC DEC AND OR XOR NOT SHL ASR SHR CMP. Each module was tested against the emulator's own code, and the whole ALU exhaustively (every alu_op, cin, a, b: 2,097,152 rows). Decisions and points to review: [alu-decisions.md](alu-decisions.md). Chip references, sourcing rules and delays: [chips.md](chips.md).

## Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `alu_op` | 4 | top | operation, table below |
| `alu_cin` | 1 | top | current C flag (ADC, SBB) |
| `alu_a`, `alu_b` | 8 | left | operands (B unused by INC DEC NOT and the shifts) |
| `alu_r` | 8 | right | result |
| `alu_c` `alu_z` `alu_n` | 1 | bottom | C, Z, N for the result |
| `alu_s` `alu_e` `alu_f` | 1 | bottom | S, E, F: valid for alu_op 0001 (CMP) |

The ALU never stores anything. The control unit chooses which flags the flag register takes: C Z N (ADD ADC SUB SBB INC DEC SHL ASR), Z N (AND OR XOR NOT SHR), or S E F (CMP).

## alu_op

| alu_op | Instr. | alu_r | alu_c |
|---|---|---|---|
| 0000 | ADD | a + b | carry |
| 0001 | SUB, CMP | a - b | borrow (= F) |
| 0010 | ADC | a + b + cin | carry |
| 0011 | SBB | a - b - cin | borrow |
| 0100 | INC | a + 1 | carry (a = FF) |
| 0101 | DEC | a - 1 | borrow (a = 00) |
| 0110 | (unused) | a + 1 + cin | carry |
| 0111 | (unused) | a - 1 - cin | borrow |
| 1000 | AND | a and b | (a7) |
| 1001 | OR | a or b | (a0) |
| 1010 | XOR | a xor b | (a7) |
| 1011 | NOT | not a | (a0) |
| 1100 | SHL | a6..a0, 0 | a7 |
| 1101 | (= ASR) | a7, a7..a1 | a0 |
| 1110 | SHR | 0, a7..a1 | (a7) |
| 1111 | ASR | a7, a7..a1 | a0 |

Values in brackets are what the hardware gives where the emulator doesn't write C. Bit by bit: op0 = sub (and ASR vs SHL/SHR for the carry), op1 = use_c, op2 = B := 01, op3 = 0 for the adder / 1 for the 74HCT138 (op2..op0). No glue logic between alu_op and the blocks.

## Modules (circuits/alu/, each made by designs/<module>.py)

| Module | Does | Chips |
|---|---|---|
| `add_sub8` (existing) | a ± b ± cin | 2 × 74HCT283, 3 × 74HCT86, 1 × 74HCT08 |
| `arith8` | add_sub8 + B or 01 (INC/DEC) | 2 × 74HCT157 |
| `logic8` | AND, OR, XOR | 2 × 74HCT08, 2 × 74HCT32, 2 × 74HCT86 |
| `shift8` | SHL/SR wiring, C select, ASR fill | 1 × 74HCT153 |
| `flags8` | Z, N; S E F from the subtractor | 1 × 74HCT27, 1 × 74HCT11, 1 × 74HCT02 |
| `rmux8` | result on a tri-state bus; NOT a | 6 × 74HCT541, 1 × 74HCT240, 1 × 74HCT138, 1 × 74HCT11 |
| `alu` | instances and tunnels only | none |

By part number: 74HCT541 × 6, 74HCT86 × 5, 74HCT08 × 3, 74HCT283 × 2, 74HCT32 × 2, 74HCT157 × 2, 74HCT11 × 2, 74HCT153 × 1, 74HCT27 × 1, 74HCT02 × 1, 74HCT138 × 1, 74HCT240 × 1. **Total: 27 chips, 12 part numbers** (26 if flags8 and rmux8 are merged, see decisions D10). With 15 % spares (at least 1 per part), buy 39.

Bill of materials, with the DIP and SOIC references to order: [bom-alu.csv](bom-alu.csv). `python designs/alu.py` rewrites it from the circuits, and `python tools/bom.py alu --check notes/bom-alu.csv` says whether it is up to date. Every chip in circuits/alu/ is in chips.md's "Parts in use" table, and no other part number is used.

## Worst-case delay, per path

From the TI datasheets of the DIP parts we will buy (chips.md, "Delays"), CL = 50 pF. Max values are at 4.5 V. Inputs a, b, cin and alu_op are stable at t = 0. Each stage counts its own worst case, so the sums are pessimistic. `python tools/timing.py` regenerates this table from chips.md.

| Path | Through (max 85 °C per stage, ns) | Typ | Max 25 °C | Max 85 °C |
|---|---|---|---|---|
| alu_r, arithmetic (bit 7) | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → S3 (66) → 541 A → Y (29) | 83 | 185 | 231 |
| alu_r, AND OR XOR | 138 A/B/C → Y (45) → 541 OE → Y (38) | 44 | 66 | 83 |
| alu_r, NOT | 138 A/B/C → Y (45) → 240 OE → Y (38) | 35 | 66 | 83 |
| alu_r, shifts | 138 A/B/C → Y (45) → 11 A/B/C → Y (35) → 541 OE → Y (38) | 55 | 94 | 118 |
| alu_f (borrow), carry out | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → C4 (58) → 86 A/B → Y (40) | 80 | 187 | 234 |
| alu_c | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → C4 (58) → 86 A/B → Y (40) → 153 I → Y (43) | 94 | 221 | 277 |
| alu_n | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → S3 (66) → 541 A → Y (29) | 83 | 185 | 231 |
| alu_z, alu_e | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → S3 (66) → 541 A → Y (29) → 27 A/B/C → Y (29) → 11 A/B/C → Y (35) | 103 | 236 | 295 |
| alu_s | 157 S → Y (36) → 86 A/B → Y (40) → 283 A/B → C4 (60) → 283 Cin → S3 (66) → 541 A → Y (29) → 27 A/B/C → Y (29) → 11 A/B/C → Y (35) → 02 A/B → Y (26) | 111 | 257 | 321 |

Typ is only a rough guide: the CD74HCT datasheets give their typical at 5 V and 15 pF (chips.md).

**Worst case: alu_s, 111 ns typ / 257 ns max 25 °C / 321 ns max 85 °C.** Over 250 ns at 85 °C: alu_s (321), alu_z and alu_e (295), alu_c (277). The result itself (alu_r, alu_n) is 231 ns, and the borrow (alu_f) 234 ns. The 74HCT283 pair is 126 ns of every arithmetic path (B to C4, then Cin to S3 or C4).

### What it means at 2 MHz

A 2 MHz step lasts 500 ns. If the registers feeding the ALU and the flag register clock on the same edge, a full step gives: SN74HCT574 clock to Q (45) + alu_s (321) + 574 setup (25) = **391 ns at 85 °C max**, with 109 ns of margin. Only a design that gives the ALU half a step (250 ns) misses.

### Fixes, if the ALU must fit in 250 ns (not applied)

Max 85 °C after the fix, in ns; costs are for the ALU alone.

| Fix | alu_r | alu_c | alu_z | alu_s | Cost |
|---|---|---|---|---|---|
| none (as built) | 231 | 277 | 295 | 321 | |
| F1. Give the ALU a full step (above) | | | | | 0 chips. A control-unit timing rule |
| F2. E from a 74HCT688 on a and b; S = nor(F, E) | 231 | 277 | 295 | 260 | +1 chip, and a new Logisim model (the 688 is missing from #TTL). Fixes CMP only |
| F3. Remove the 2 × 74HCT157: the control unit puts 01 on B for INC/DEC | 195 | 241 | 259 | 285 | −2 chips, but 01 must come from somewhere else (e.g. a 541 with tied inputs on the B operand bus: +1) |
| F4. Carry select on the high nibble (3rd 283 for cin = 1, 2 × 157 pick the result) | 201 | 255 | 265 | 291 | +3 chips |
| F3 + F4 | 165 | 219 | 229 | 255 | +2 chips (with the 01 source) |
| F2 + F3 + F4 | 165 | 219 | 229 | 202 | +3 chips and the 688 model |
| F5. Store the flags one step after the result (or a separate CMP step) | | | | | 0 chips; one more microstep per flag-setting instruction, unless it overlaps the next fetch |

My recommendation is F1, which is the normal arrangement and costs nothing. If the control unit can't give the ALU a full step, F5 costs nothing in chips. F2 to F4 are only worth it together, for +3 chips.

## Tests (hbc2 vectors → Logisim headless, logitest.py)

| Circuit | Rows | Result |
|---|---|---|
| add_sub8 | 524,288 (all) | pass (earlier session) |
| logic8 | 65,536 (all) | 65,536 / 65,536 |
| arith8 | 1,048,576 (all) | 1,048,576 / 1,048,576 |
| shift8 | 2,048 (all) | 2,048 / 2,048 |
| flags8 | 512 (all) | 512 / 512 |
| rmux8 | 65,760 (sampled, 60 input bits) | 65,760 / 65,760 |
| alu | 2,097,152 (all: op, cin, a, b) | **2,097,152 / 2,097,152** (4 chunks, about 10 min each) |

The expected values come from the emulator. ADD/ADC/SUB/SBB use `hbc2_core::alu`. For the other instructions, `hbc2 vectors` runs the real CPU on the instruction and reads the register and the flags back. The emulator was not changed. Flags an instruction doesn't write are don't care.

Checks: circcheck and conventions report 0 errors, 0 warnings on every file. circcheck gained a rule: a Ground/Power/Constant sharing a net with a driver is an error. That rule caught a short in an early arith8 layout.

## What is left

- Gianni's review of the items in alu-decisions.md (SHR wording in the manual, unsigned CMP, the tri-state bus, the 27 vs 26 chips, the alu_op encoding).
- Decide the ALU's time budget in the control unit (F1 or F5 above), since three flags are over 250 ns at 85 °C max.
- Look at the layouts in Logisim. I only saw approximate renders, so labels may overlap in places.
- Next sections, out of scope tonight: the flag register (with a write mask per flag group) and the control unit that produces alu_op.
- cpu.circ still loads only alu and add_sub8 as libraries. Loading alu now pulls in the other modules through alu.circ.
