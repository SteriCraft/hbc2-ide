# TTL parts: pinout quirks and wiring choices

## 74283: 4-bit adder (74HCT283)

Placed facing east at (X, Y): top pins are at y = Y-30, bottom pins at Y+30, and x = X + 10 + 20i.

| Edge | Pins, left to right |
|---|---|
| top | VCC B3 A3 ∑3 A4 B4 ∑4 C4 |
| bottom | ∑2 B2 A2 ∑1 A1 B1 C0(CIN) GND |

circgen accepts `S1` as an alias for `∑1`.

A, B and S bits alternate on both edges. If one 4-bit splitter feeds each bus of a chip, 1-bit wires have to cross each other, whatever the layout. **Decision (user, 2026-11-04):** keep 1-bit taps on vertical bus lines for the 74283 (add8_ttl layout). Pins leave on nested rails toward the left, so only bus lines cross bit wires.

Stack two chips with the high nibble above, so the carry C4 → CIN is a short jog. In add8 the high nibble is U1 (top) and the low nibble is U2: the numbering follows position, not bit order.
