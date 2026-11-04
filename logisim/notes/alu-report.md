# ALU: report (unattended run, night of 2026-11-04)

The `alu` section now covers every ALU instruction of the HBC-2: ADD ADC SUB SBB INC DEC AND OR XOR NOT SHL ASR SHR CMP. Each module was tested against the emulator's own code, and the whole ALU exhaustively (every alu_op, cin, a, b: 2,097,152 rows). Decisions and points to review: [alu-decisions.md](alu-decisions.md). Chip sourcing and delays: [chips.md](chips.md).

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

By part number: 74HCT541 × 6, 74HCT86 × 5, 74HCT08 × 3, 74HCT283 × 2, 74HCT32 × 2, 74HCT157 × 2, 74HCT11 × 2, 74HCT153 × 1, 74HCT27 × 1, 74HCT02 × 1, 74HCT138 × 1, 74HCT240 × 1. **Total: 27 chips** (26 if flags8 and rmux8 are merged, see decisions D10).

## Worst-case delay, per path

Typical values, 74HCT at 4.5 V and 50 pF (chips.md). Inputs a, b, cin and alu_op are stable at t = 0.

| Path | Through | Typ. |
|---|---|---|
| alu_r, arithmetic | op2 → 157 select (18) → add_sub8 B xor (14) → 283 low, B to C4 (21) → 283 high, Cin to Σ (23) → 541 (14) | **90 ns** |
| alu_r, logic | a → 08/32/86 (14) → 541 (14) | 28 ns |
| alu_r, NOT | a → 240 (12) | 12 ns |
| alu_r, shifts | op → 138 (17) → 7411 (16) → 541 enable (16) | 49 ns |
| alu_n | = alu_r bit 7 | 90 ns |
| alu_z, alu_e | alu_r (90) → 7427 (14) → 7411 (16) | 120 ns |
| alu_s | alu_z (120) → 7402 (11) | **131 ns** |
| alu_f | add_sub8 carry: … 283 high Cin to C4 (20) → 86 (14) | 87 ns |
| alu_c | alu_f path (87) → 153 data (19) | 106 ns |

Worst case: **alu_s ≈ 131 ns typical**, under the 200 ns target. With 25 °C maximum values (about 1.5 ×) it is about 200 ns, and over the full temperature range it would be more. A faster option, if needed: make S from the borrow and a 74HCT688-style equality, or simply give CMP its own microstep.

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
- Check the delay figures against the exact datasheets of the parts to be bought (the 74HCT283 first).
- Look at the layouts in Logisim. I only saw approximate renders, so labels may overlap in places.
- Next sections, out of scope tonight: the flag register (with a write mask per flag group) and the control unit that produces alu_op.
- cpu.circ still loads only alu and add_sub8 as libraries. Loading alu now pulls in the other modules through alu.circ.
