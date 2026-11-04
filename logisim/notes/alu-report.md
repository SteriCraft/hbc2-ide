# ALU: report (night of 2026-11-04, timing fixes the same day)

The `alu` section covers every ALU instruction of the HBC-2: ADD ADC SUB SBB INC DEC AND OR XOR NOT SHL ASR SHR CMP. Each module was tested against the emulator's own code, and the whole ALU exhaustively (every alu_op, cin, a, b: 2,097,152 rows). Decisions and points to review: [alu-decisions.md](alu-decisions.md). Chip references, sourcing rules and delays: [chips.md](chips.md).

After the datasheet timing, Gianni approved fixes F1–F4 (the ALU gets a full step, plus three hardware changes). They are built: CMP flags from 2 × 74HCT85 (cmp8), INC/DEC with b = 01 from the control unit (arith8 removed), and a carry select in add_sub8.

## Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `alu_op` | 4 | top | operation, table below |
| `alu_cin` | 1 | top | current C flag (ADC, SBB) |
| `alu_a`, `alu_b` | 8 | left | operands. **INC and DEC need alu_b = 01** (the control unit drives it). B is unused by NOT and the shifts |
| `alu_r` | 8 | right | result |
| `alu_c` `alu_z` `alu_n` | 1 | bottom | C, Z, N for the result |
| `alu_s` `alu_e` `alu_f` | 1 | bottom | S, E, F: a > b, a = b, a < b, unsigned, for any alu_op (CMP stores them) |

The ALU never stores anything. The control unit chooses which flags the flag register takes: C Z N (ADD ADC SUB SBB INC DEC SHL ASR), Z N (AND OR XOR NOT SHR), or S E F (CMP).

## alu_op

| alu_op | Instr. | alu_r | alu_c |
|---|---|---|---|
| 0000 | ADD; INC with b = 01 | a + b | carry |
| 0001 | SUB, CMP; DEC with b = 01 | a - b | borrow |
| 0010 | ADC | a + b + cin | carry |
| 0011 | SBB | a - b - cin | borrow |
| 01xx | (aliases of 00xx: op2 has no effect on arithmetic codes, 01xx = 00xx) | | |
| 1000 | AND | a and b | (a7) |
| 1001 | OR | a or b | (a0) |
| 1010 | XOR | a xor b | (a7) |
| 1011 | NOT | not a | (a0) |
| 1100 | SHL | a6..a0, 0 | a7 |
| 1101 | (= ASR) | a7, a7..a1 | a0 |
| 1110 | SHR | 0, a7..a1 | (a7) |
| 1111 | ASR | a7, a7..a1 | a0 |

Values in brackets are what the hardware gives where the emulator doesn't write C. Bit by bit: op0 = sub (and ASR vs SHL/SHR for the carry), op1 = use_c, op2 = only the 138's C input, op3 = 0 for the adder / 1 for the 74HCT138 (op2..op0). No glue logic between alu_op and the blocks.

## Modules (circuits/alu/, each made by designs/<module>.py)

| Module | Does | Chips |
|---|---|---|
| `add_sub8` | a ± b ± cin, carry select on bits 7..4 | 3 × 74HCT283, 3 × 74HCT86, 2 × 74HCT157, 1 × 74HCT08 |
| `logic8` | AND, OR, XOR | 2 × 74HCT08, 2 × 74HCT32, 2 × 74HCT86 |
| `shift8` | SHL/SR wiring, C select, ASR fill | 1 × 74HCT153 |
| `flags8` | Z, N | 1 × 74HCT27, 1 × 74HCT11 |
| `cmp8` | S, E, F from a and b | 2 × 74HCT85 |
| `rmux8` | result on a tri-state bus; NOT a | 6 × 74HCT541, 1 × 74HCT240, 1 × 74HCT138, 1 × 74HCT11 |
| `alu` | instances and tunnels only | none |

By part number: 74HCT541 × 6, 74HCT86 × 5, 74HCT08 × 3, 74HCT283 × 3, 74HCT32 × 2, 74HCT157 × 2, 74HCT11 × 2, 74HCT85 × 2, 74HCT153 × 1, 74HCT27 × 1, 74HCT138 × 1, 74HCT240 × 1. **Total: 29 chips, 12 part numbers** (28 if flags8 and rmux8 are merged, see decisions D10). With 15 % spares (at least 1 per part), buy 41. Outside the ALU, INC/DEC need a 01 source on the B operand path (likely +1 74HCT541, D16).

Bill of materials, with the DIP and SOIC references to order: [bom-alu.csv](bom-alu.csv). `python designs/alu.py` rewrites it from the circuits, and `python tools/bom.py alu --check notes/bom-alu.csv` says whether it is up to date. Every chip in circuits/alu/ is in chips.md's "Parts in use" table, and no other part number is used.

## Worst-case delay, per path

From the TI datasheets of the DIP parts (chips.md, "Delays"), CL = 50 pF. Max values are at 4.5 V. Inputs a, b, cin and alu_op are stable at t = 0. Each stage counts its own worst case, so the sums are pessimistic. A multiplexer output counts the later of its data and its select. `python tools/timing.py` regenerates this table.

| Path | Through (max 85 °C per stage, ns) | Typ | Max 25 °C | Max 85 °C |
|---|---|---|---|---|
| alu_r, arithmetic (bit 7) | 08 A/B → Y (30) → 86 A/B → Y (40) → 283 Cin → C4 (58) → 157 S → Y (36) → 541 A → Y (29) | 75 | 154 | 193 |
| alu_r, AND OR XOR | 138 A/B/C → Y (45) → 541 OE → Y (38) | 44 | 66 | 83 |
| alu_r, NOT | 138 A/B/C → Y (45) → 240 OE → Y (38) | 35 | 66 | 83 |
| alu_r, shifts | 138 A/B/C → Y (45) → 11 A/B/C → Y (35) → 541 OE → Y (38) | 55 | 94 | 118 |
| adder carry / borrow | 08 A/B → Y (30) → 86 A/B → Y (40) → 283 Cin → C4 (58) → 157 S → Y (36) → 86 A/B → Y (40) | 75 | 163 | 204 |
| alu_c | 08 A/B → Y (30) → 86 A/B → Y (40) → 283 Cin → C4 (58) → 157 S → Y (36) → 86 A/B → Y (40) → 153 I → Y (43) | 89 | 197 | 247 |
| alu_n | 08 A/B → Y (30) → 86 A/B → Y (40) → 283 Cin → C4 (58) → 157 S → Y (36) → 541 A → Y (29) | 75 | 154 | 193 |
| alu_z | 08 A/B → Y (30) → 86 A/B → Y (40) → 283 Cin → C4 (58) → 157 S → Y (36) → 541 A → Y (29) → 27 A/B/C → Y (29) → 11 A/B/C → Y (35) | 95 | 205 | 257 |
| alu_s, alu_f | 85 A/B → A>B, A<B (46) → 85 cascade in → A>B, A<B (38) | 27 | 67 | 84 |
| alu_e | 85 A/B → A=B (50) → 85 cascade in → A=B (39) | 30 | 71 | 89 |

Typ is only a rough guide: the CD74HCT datasheets give their typical at 5 V and 15 pF (chips.md).

**Worst case: alu_z, 95 ns typ / 205 ns max 25 °C / 257 ns max 85 °C.** The budget is the full 500 ns step (F1), not 250 ns: the 250 ns figure was an arbitrary early target, so 257 ns is not a miss (Gianni, 2026-11-04). Every other output is at or under 247 ns (alu_c). The arithmetic result is 193 ns, and S/E/F 84–89 ns.

Before the fixes, the worst case was alu_s at 111 / 257 / 321 ns. My estimate for the fixes was 229 ns for Z, and the build gives 257 ns. Once the 157 is gone from the B path, the carry-in logic (74HCT08 then 74HCT86, 70 ns at 85 °C) reaches U6's carry later than b does (40 ns), and I had left that path out.

### At 2 MHz

The ALU gets a full 500 ns step (F1, approved). Register clock to Q (SN74HCT574, 45) + alu_z (257) + setup (25) = **327 ns at 85 °C max**, so 173 ns of margin.

### Getting alu_z under 250 ns (not needed, not applied)

Kept for the record. Gianni decided on 2026-11-04 to leave alu_z as it is.

| Option | alu_z max 85 °C | Cost |
|---|---|---|
| A. The control unit gives the ALU the adder's carry in ready-made (sub xor (use_c and C), computed in an earlier step), instead of alu_cin | 229 | −1 74HCT08 in the ALU. The gates move to the control unit, which must have them ready at the start of the step |
| B. Regulated 5 V ±5 % supply (4.75 V minimum instead of 4.5 V) | about 245 | no change. The datasheets only give 4.5 V and 6 V, so this is an estimate |

Decision: neither. With the full step there are 173 ns of margin.

## Tests (hbc2 vectors → Logisim headless, logitest.py)

| Circuit | Rows | Result |
|---|---|---|
| add_sub8 (carry select) | 524,288 (all) | 524,288 / 524,288 |
| logic8 | 65,536 (all) | 65,536 / 65,536 |
| shift8 | 2,048 (all) | 2,048 / 2,048 |
| flags8 (Z N) | 256 (all) | 256 / 256 |
| cmp8 | 65,536 (all) | 65,536 / 65,536 |
| rmux8 | 65,760 (sampled, 60 input bits) | 65,760 / 65,760 |
| alu | 2,097,152 (all: op, cin, a, b) | **2,097,152 / 2,097,152** (4 chunks) |

The expected values come from the emulator. For every instruction, `hbc2 vectors` runs the real CPU on the instruction and reads the register and the flags back. The emulator was not changed. Flags an instruction doesn't write are don't care. On every alu row with b = 01, the generator also checks that the emulator's INC/DEC give the same result and flags as ADD/SUB (D16).

Checks: circcheck and conventions report 0 errors, 0 warnings on every file. circcheck gained a rule: a Ground/Power/Constant sharing a net with a driver is an error. That rule caught a short in an early arith8 layout.

## What is left

- Gianni's review of the items in alu-decisions.md (SHR wording in the manual, unsigned CMP, the tri-state bus, the 29 vs 28 chips, the alu_op encoding).
- The control unit must drive alu_b = 01 for INC/DEC (D16), and give the ALU a full step (F1).
- Look at the layouts in Logisim. I only saw approximate renders, so labels may overlap in places.
- Next sections, out of scope tonight: the flag register (with a write mask per flag group) and the control unit that produces alu_op.
- cpu.circ still loads only alu and add_sub8 as libraries. Loading alu now pulls in the other modules through alu.circ.
