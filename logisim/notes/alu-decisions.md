# ALU: decisions (unattended run of 2026-11-04)

Gianni asked me to finish the ALU overnight and to write down every choice I made without asking. Each entry gives the question, the options, the choice and the reason. The list of points to review is at the end. The summary is in [alu-report.md](alu-report.md).

## What the emulator does (read from hbc2-core/src/cpu.rs)

| Instr. | Result | Flags written |
|---|---|---|
| ADD / ADC | a + b (+ C) | C = carry, Z, N |
| SUB / SBB | a - b (- C) | C = borrow, Z, N |
| INC / DEC | a + 1 / a - 1 (`overflowing_add/sub`) | C = carry (a = FF) / borrow (a = 00), Z, N |
| AND / OR / XOR | a op b | Z, N |
| NOT | not a | Z, N |
| SHL | a << 1 | C = bit 7 of a, Z, N |
| ASR | arithmetic: (a >> 1) with bit 7 kept | C = bit 0 of a, Z, N |
| SHR | **logical**: a >> 1, bit 7 = 0 | Z, N (C untouched) |
| CMP | none | S = a > b, E = a = b, F = a < b, **unsigned** |

Z = result is 0, N = bit 7 of the result, for every instruction that writes them.

## D1. Where the expected values come from

Question: the ALU instructions other than ADD/SUB are written inline in `cpu.rs`, so `hbc2_core::alu` has no function for them. The conventions say to move such code into a pure function first, but tonight's rule is "never change the emulator".
Options: (a) copy the emulator's expressions into the vector generator; (b) move the code into `alu.rs`, which changes the emulator; (c) run the real CPU on each instruction.
Choice: **(c)**. `hbc2 vectors` (hbc2-cli/src/vectors.rs, test tooling, not the emulator) now loads one instruction in Reg mode, sets A, B and the carry, calls `Cpu::tick`, and reads the register and the flags back. It also checks that the flags missing from the instruction table (`Opcode::info().flags`) are left untouched. Those flags are written `x` (don't care) in the vectors, because the control unit won't store them.
Reason: the reference really is the emulator's code, with nothing copied and nothing changed.

## D2. Exhaustive limit raised to 2^21, chunked runs

Question: the full ALU has 4 + 8 + 8 + 1 = 21 input bits, and the old limit was 2^20.
Choice: `MAX_BITS = 21`. `logitest.check_emulator` now splits a vector file into chunks of 524,288 rows, the size known to work in Logisim.
Reason: the prompt asks for an exhaustive test of every alu_op, A, B and C.

## D3. logic8 contains AND, OR and XOR, but not NOT

Options for NOT A: (a) 2 × 74HCT04, i.e. 8 inverters; (b) a XOR with FF, which needs B forced to FF, so 2 more chips; (c) an inverting tri-state buffer, the 74HCT240, as NOT's source on the result bus.
Choice: **(c)**. The result multiplexer needs one buffer per source anyway, and the 240 is that buffer and the inverter at once, so NOT costs no extra chip.

## D4. Layout of logic8

Three columns (AND, OR, XOR), each with two chips stacked, in the add_sub8 style: nested rails, vertical lines with 1-bit taps. a and b reach each column through tunnels. Input pins sit at x = 220 so their labels show; Gianni had moved add_sub8 and alu for that same reason. circgen now has `Circuit.move(dx, dy)`, and add_sub8.py and alu.py reproduce his shifted layouts.

## D5. INC / DEC: B := 01 through 2 × 74HCT157 (arith8)

Options: (a) force B to 01 with 2 × 74HCT157, as the prompt suggests; (b) force B to 00 and the carry in to 1, which needs an OR gate on cin, since add_sub8 computes carry in = sub xor (use_c and cin); (c) gate B with 2 × 74HCT08 and an inverted control.
Choice: **(a)**. The 157's A inputs take b, its B inputs are tied to 0 or 1 (Ground / Power), and SELECT = `arith8_one`, with no strobe. Then INC = a + 01 and DEC = a - 01 with use_c = 0, and the carry/borrow out is exactly the emulator's INC/DEC C (a = FF / a = 00).
Reason: same chip count as (b) and (c), no extra gate, and only the select line toggles.

## D6. CMP uses the subtractor, so there is no cmp8 module

Options: (a) 2 × 74HCT85 magnitude comparators in cascade; (b) the 74HCT688 for E, which is missing from Logisim and doesn't give S/F; (c) run CMP as SUB and derive the flags from it.
Choice: **(c)**. The emulator compares unsigned, so after a - b: F = borrow, E = Z of the result, S = nor(borrow, Z). This costs one 74HCT02 gate, in flags8. CMP uses the same alu_op as SUB (0001): the ALU always computes the same thing, and the control unit stores S E F instead of r C Z N.
Reason: saves 2 chips. Cost: S comes last in the timing (321 ns at 85 °C max, see D13).

## D7. Result multiplexer: a tri-state bus (rmux8)

Options: (a) 8 × 74HCT151 (8-to-1 per bit), plus 2 × 74HCT04 for NOT, for 10 chips; (b) 74HCT153/157 trees, more than 10 chips; (c) one tri-state buffer per source on an internal bus, plus a 74HCT138 decoder.
Choice: **(c)**: 6 × 74HCT541 (sum, and, or, xor, shl, sr), 1 × 74HCT240 (NOT a, which inverts), 1 × 74HCT138, 1 × 74HCT11 gate, so 9 chips. Every alu_op enables exactly one buffer, so the bus never floats.
Reason: fewest chips, and the shifts are free because they are wiring into their buffer. Cost: while alu_op changes, two buffers can drive the bus together for a few ns. That is normal for a homebrew CPU bus and harmless at 2 MHz, but it is on the review list.

## D8. Z: 74HCT27 + 74HCT11, as before

Options: 74HCT4078 (8-input NOR), missing from Logisim; 74HCT30 (8-input NAND), which needs 8 inverters; a NOR tree of 74HCT02s, 3 chips. Kept: 3 × NOR3 (74HCT27), then AND3 (74HCT11), 2 chips. Gianni confirmed on 2026-11-04: no 4078 model. Saving one chip isn't worth an HC part and a new model type.

## D9. alu_op encoding (4 bits, no decoding glue)

The ISA opcodes are in alphabetical order (ADC 01, ADD 02, AND 03 ...), so no subset of opcode bits gives a cheap ALU code. The control unit will produce alu_op from its own decoder or microcode ROM anyway, where 4 output bits cost nothing. So I chose the bits that drive the blocks directly:

| alu_op | op3 = 0: arithmetic (sum on alu_r) | | alu_op | op3 = 1 (138 decodes op2..op0) |
|---|---|---|---|---|
| 0000 | ADD | | 1000 | AND |
| 0001 | SUB, and CMP | | 1001 | OR |
| 0010 | ADC | | 1010 | XOR |
| 0011 | SBB | | 1011 | NOT |
| 0100 | INC | | 1100 | SHL |
| 0101 | DEC | | 1101 | (= ASR) |
| 0110 | (a + 1 + cin) | | 1110 | SHR |
| 0111 | (a - 1 - cin) | | 1111 | ASR |

Wiring: op0 = add_sub8 sub = shift8_asr, op1 = use_c, op2 = arith8_one, op3 = shift8_sel = enable of the sum buffer (active low) = G1 of the 138. The codes in brackets are unused, but they are defined and tested: no code leaves the bus floating.

## D10. Module split

- `logic8`: gates only (D3).
- `arith8`: add_sub8 + the B / 01 multiplexer (D5).
- `shift8`: the shifted buses (wiring) and one 74HCT153. Half 1 picks C (adder carry / a7 for SHL / a0 for ASR), half 2 makes the ASR fill bit. C select sits here because 2 of its 3 sources are shift carries.
- `flags8`: Z, N and the CMP flags (D6).
- `rmux8`: the result bus (D7).
- `alu`: only instances and tunnels, no chips.
The shift-right enable needs a 3-input AND of three decoder outputs. It has its own 74HCT11 in rmux8, although flags8's 74HCT11 has two spare gates. Merging flags8 into rmux8 would save that chip (27 → 26). I kept the modules separate for readability; this is on the review list.

## D11. Don't-care outputs in the tests

A flag the emulator doesn't write for an instruction is `x`: C for AND/OR/XOR/NOT/SHR, S/E/F everywhere except alu_op 0001 (CMP). Everything the emulator writes is checked. For 0001, r C Z N come from SUB and S E F from CMP, on the same a, b. This doesn't loosen the test: the control unit doesn't store those flags.

## D12. rmux8 is tested on sampled rows

It has 60 input bits, and there is no emulator function for a multiplexer. Its reference is the alu_op table above, in `hbc2 vectors rmux8`: 16 ops × (one-hot 00/FF patterns + 4,096 random independent sources) = 65,760 rows. The emulator check for the result path is the exhaustive `alu` test.

## D13. Timing

Redone on 2026-11-04 from the TI datasheets of the DIP parts (chips.md "Delays", `tools/timing.py`). The worst case is the S flag: 111 ns typ, 257 ns max at 25 °C, **321 ns max at 85 °C**. Z/E (295) and C (277) are also over 250 ns at 85 °C. That is fine if the ALU gets a full 2 MHz step (391 ns with the registers' clock-to-Q and setup), and too slow if it gets half a step. The report lists fixes and their cost, none applied. See the review list.

## D14. Layout margin

Gianni moved add_sub8 and alu so the input pin labels show. New circuits put their input pins at x ≥ 220 (300 in rmux8, whose labels are longer), and circgen's `Circuit.move` keeps his shifts in the scripts.

## Need Gianni's review

1. **SHR, manual vs emulator.** The manual (footnote 6) says "SHR is equivalent to ASR, but the bit shifted to the right is discarded, instead of being carried over". Read literally, SHR would keep bit 7 like ASR. The emulator does a *logical* shift (bit 7 = 0) and leaves C alone. I followed the emulator. A logical SHR is the useful one, since ASR already exists, so the manual's wording is probably what needs fixing.
2. **CMP is unsigned** in the emulator (`a > b` on u8). The manual only says `FR = R1 ? R2`. Signed compares would need different logic (S/F from N xor V); today they need a software trick.
3. **Tri-state result bus** (D7): brief overlap between buffers when alu_op changes. It's fine at 2 MHz, but if you'd rather avoid any contention, an 8 × 74HCT151 multiplexer costs about 2 more chips.
4. **27 chips; 26 if flags8 and rmux8 are merged** (D10). Tell me if you prefer fewer chips over separate modules.
5. **alu_op encoding** (D9): check that it suits the control unit you have in mind. CMP shares 0001 with SUB.
6. **ALU time budget**: with datasheet maxima at 85 °C, S, Z/E and C need 277 to 321 ns, so the ALU needs a full 500 ns step at 2 MHz (391 ns with register timing). Decide this when the control unit is designed; the fixes F1 to F5 and their cost are in alu-report.md. I recommend F1 (a full step, 0 chips).
7. **`alu` ports changed**: alu_s (result) became alu_r, alu_s is now the Superior flag, and alu_sub/alu_use_c became alu_op. Nothing instantiates alu yet (`tools/usages.py alu`), so nothing needs rewiring.
8. ~~The 74HCT27 is the hardest part to find.~~ Wrong: Gianni's Octopart check (2026-11-04) shows all 12 part numbers are easy to buy, the 74HCT27 included. The references to order are in chips.md and bom-alu.csv.
9. No emulator bug found. Each ALU instruction writes exactly the flags listed in the instruction table, which `hbc2 vectors` checks on every row.
