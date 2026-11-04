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
- add_sub8: U1/U3 (86) xor B with sub; U5 (08) use_c and cin; U6 (86) carry in = sub xor (use_c and cin), c = carry out xor sub.
- alu: U1 (27) nor3(s0,s1,gnd), nor3(s2..s4), nor3(s5..s7); U2 (11) and3 of those = Z. No 8-input NOR in #TTL; the 7430 (8-input NAND) would need 9 inverters, 3 chips instead of 2.
