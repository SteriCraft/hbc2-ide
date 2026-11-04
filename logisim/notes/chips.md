# Chips: what to buy, how fast they are, what Logisim has

The parts used or considered for the HBC-2, so the next module doesn't have to search again. Pinouts and wiring choices are in [ttl-chips.md](ttl-chips.md).

## Rules (Gianni, 2026-11-04)

- Every chip must be sold as a **74HCTxx**, the CMOS family with TTL-compatible inputs.
- Both DIP and **SMD (SOIC)** are fine; it doesn't have to be through-hole.
- If a chip is missing from Logisim's `#TTL` library, pick another real part instead of building it from gates.

## Parts in use

Delays are **typical values at VCC = 4.5 V, CL = 50 pF, 25 °C**, as given in Nexperia/NXP 74HC_HCT datasheets. At 5 V they are about 10 % shorter. The datasheet's 25 °C maximum is roughly 1.5 × typical, and the 85 °C maximum roughly 1.9 ×. I didn't open every datasheet: values marked ~ are estimates for the family and need checking before the timing is final.

| Part | Function | Logisim | Typ. delay (4.5 V) | Packages / makers seen | Used in |
|---|---|---|---|---|---|
| 74HCT02 | quad 2-in NOR | 7402 | ~11 ns | Nexperia SO-14, TI DIP | flags8 |
| 74HCT08 | quad 2-in AND | 7408 | ~14 ns | Nexperia, TI (DIP and SO) | add_sub8, logic8 |
| 74HCT11 | triple 3-in AND | 7411 | 16 ns (max 24) | Nexperia 74HCT11N DIP / D SO-14, TI CD74HCT11E | flags8, rmux8 |
| 74HCT27 | triple 3-in NOR | 7427 | ~14 ns | TI CD74HCT27E DIP (distributors), Nexperia 74HCT27D SO-14; Tayda DIP out of stock | flags8 |
| 74HCT32 | quad 2-in OR | 7432 | ~14 ns | Nexperia, TI | logic8 |
| 74HCT86 | quad 2-in XOR | 7486 | ~14 ns | Nexperia, TI CD74HCT86 | add_sub8, logic8 |
| 74HCT138 | 3-to-8 decoder | 74138 | 17 ns (A to Y) | Nexperia, TI | rmux8 |
| 74HCT153 | dual 4-to-1 mux | 74153 | 19 ns (data), ~20 ns (select) | Nexperia 74HCT153D SO-16, TI | shift8 |
| 74HCT157 | quad 2-to-1 mux | 74157 | 16 ns (data), ~18 ns (select) | Nexperia, TI SN74HCT157N DIP | arith8 |
| 74HCT240 | octal inverting buffer, tri-state | 74240 | ~12 ns, enable ~15 ns | TI CD74HCT240E DIP (DigiKey), Nexperia | rmux8 (NOT) |
| 74HCT283 | 4-bit adder | 74283 | ~21 ns (A/B to C4), ~20 ns (Cin to C4), ~23 ns (Cin to S) | TI CD74HCT283E DIP, Nexperia 74HCT283D SO-16 | add_sub8 |
| 74HCT541 | octal buffer, tri-state | 74541 | ~14 ns, enable ~16 ns | TI SN74HCT541N DIP (DigiKey: active), CD74HCT541E, Nexperia | rmux8 |

Sourcing notes:
- The 74HCT27 is the least common part on this list. If it can't be found, Z can be computed differently (see alu-decisions.md D8), but every option found so far costs one more chip.
- On TI parts, check the exact suffix: SN74HCT541NE4 shows as obsolete on Arrow while SN74HCT541N is active.

## In Logisim but not a 74HCT part (don't use)

- **74181 / 74182** (4-bit ALU / carry lookahead): sold only as LS/F/S TTL, which are obsolete. No HCT version found. Two 74181s would have done add, sub, the logic operations and A = B on their own.
- **74381** (4-bit ALU): S/LS only.
- Some parts can't be checked from their Logisim name alone (7413, 7418, 7419, 7421, 7451, 7454, 74670, 747266). Check them before use.

## Real 74HCT parts missing from Logisim

- **74HCT688** (8-bit equality comparator): would give A = B in one chip. CMP uses the subtractor instead (D6).
- **74HCT4078** (8-input NOR/OR): would give Z in one chip instead of 74HCT27 + 74HCT11.
- **74HCT540** (inverting 541): the 74HCT240 does the same job here.
- **74HCT154** (4-to-16 decoder): would remove the 7411 gate behind the shift-right enable.
