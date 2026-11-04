# TTL parts: pinout quirks and wiring choices

## 74283: 4-bit adder (74HCT283)

Placed facing east at (X, Y): top pins are at y = Y-30, bottom pins at Y+30, and x = X + 10 + 20i.

| Edge | Pins, left to right |
|---|---|
| top | VCC B3 A3 ∑3 A4 B4 ∑4 C4 |
| bottom | ∑2 B2 A2 ∑1 A1 B1 C0(CIN) GND |

circgen accepts `S1` as an alias for `∑1`.

A, B and S bits alternate on both edges. If one 4-bit splitter feeds each bus of a chip, 1-bit wires have to cross each other, whatever the layout. **Decision (user, 2026-11-04):** keep 1-bit taps on vertical bus lines for the 74283 (add_sub8 layout, circuits/alu/add_sub8.circ). Pins leave on nested rails toward the left, so only bus lines cross bit wires.

Stack two chips with the high nibble above, so the carry C4 → CIN is a short jog. In add_sub8 the high nibble is U2 and the low nibble U4 (U1/U3 are the XOR chips to their left): the numbering follows position, not bit order.

## Gate chips: 7486, 7408, 7427, 7411 (74HCT86/08/27/11)

Same 14-pin outline. Placed facing east at (X, Y): top pins at y = Y-30, bottom at Y+30, x = X + 10 + 20i.

| Chip | top, left to right | bottom, left to right |
|---|---|---|
| 7486, 7408 (2-input) | VCC 4B 4A 4Y 3B 3A 3Y | 1A 1B 1Y 2A 2B 2Y GND |
| 7427, 7411 (3-input) | VCC 1C 1Y 3C 3B 3A 3Y | 1A 1B 2A 2B 2C 2Y GND |

Inputs and outputs alternate on each edge, so as with the 74283 one splitter per bit group forces crossings. **Decision (user, 2026-11-04):** the 1-bit tap exception covers every TTL chip whose pins mix inputs and outputs. Every pin leaves left on a nested rail toward vertical lines (buses with 1-bit taps, or 1-bit lines such as `sub`).

On the 7427/7411, 1Y sits between 1C and 3C, so 1C can't take a signal from above without crossing: alu grounds it (rail ending on a Ground facing west).

Uses:
- add_sub8: U1/U5 (86) xor B with sub; U8 (08) use_c and cin; U9 (86) carry in = sub xor (use_c and cin), c = carry out xor sub. Carry select: U6 (283) bits 3..0, U2 / U3 (283) bits 7..4 with carry in 0 / 1, U4 / U7 (157) pick the high sum / carry out with U6's C4.
- flags8: U1 (27) nor3(r0,r1,r2), nor3(r3..r5), nor3(r6,r7,gnd); U2 (11) and3 of those = Z. No 8-input NOR in #TTL; the 7430 (8-input NAND) would need 9 inverters, 3 chips instead of 2.

## Pinouts used by the ALU modules (placed facing east at (X, Y))

| Chip | top, left to right | bottom, left to right | Notes |
|---|---|---|---|
| 7402 | VCC 4Y 4B 4A 3Y 3B 3A | 1Y 1A 1B 2Y 2A 2B GND | outputs first, unlike the 7400 (no longer used by the ALU) |
| 7485 | VCC A3 B2 A2 A1 B1 A0 B0 | B3 A<B A=B A>B A>B A=B A<B GND | pins 2-4 are the cascade **inputs**, 5-7 the outputs: same names, so wire by pin number (chip[2] ...). Alone or lowest chip: A=B in = 1, the other two 0 |
| 74157 | VCC nSTROBE 4A 4B 4Y 3A 3B 3Y | SELECT 1A 1B 1Y 2A 2B 2Y GND | Y = B when SELECT = 1; nSTROBE = 1 forces Y = 0 |
| 74153 | VCC n2E S0 2D3 2D2 2D1 2D0 2Y | n1E S1 1D3 1D2 1D1 1D0 1Y GND | **80 px tall**: bottom pins at Y + 50. S1 is the high select bit (checked by test) |
| 74138 | VCC nY0 .. nY6 | A B C nG2A nG2B G1 nY7 GND | C is the high bit |
| 74541 | VCC nOE2 Y1 .. Y8 | nOE1 A1 .. A8 GND | inputs on one edge, outputs on the other: one 8-bit splitter each side |
| 74240 | VCC n2G n1Y1 2A4 n1Y2 2A3 n1Y3 2A2 n1Y4 2A1 | n1G 1A1 n2Y4 1A2 n2Y3 1A3 n2Y2 1A4 n2Y1 GND | inverting, two 4-bit halves; mixed edges, so rail style |

Uses: add_sub8 U4/U7 (157: carry select), shift8 U1 (153), flags8 U1-U2, cmp8 U1-U2 (85), rmux8 U1 (138), U2 (11), U3 (240), U4-U9 (541).
