#!/usr/bin/env python3
"""8-bit unsigned compare (CMP flags) -> circuits/alu/cmp8.circ

Two 74HCT85 magnitude comparators in series cascade:

    U1 (bits 3..0)  cascade inputs A<B = 0, A=B = 1, A>B = 0
    U2 (bits 7..4)  cascade inputs = U1's outputs

    cmp8_s = U2 A>B     a > b
    cmp8_e = U2 A=B     a = b
    cmp8_f = U2 A<B     a < b

The emulator's CMP compares unsigned, so this is exactly CMP's S E F, on a and
b directly: the flags don't wait for the adder (alu-decisions.md D15).

The 7485 has two pins named A>B (and A=B, A<B): pins 2-4 are the cascade
inputs, 5-7 the outputs, so the chips are wired by pin number.

Layout: the chips side by side, low nibble on the left. Each has its own a
and b lines, fed over the top; data pins leave on nested rails. The cascade
runs under the chips as three nested wires, and the flags drop straight down
from U2.

    python cmp8.py           # write, check
    python cmp8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python cmp8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, rails_left  # noqa: E402

M = "cmp8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

X_IN, Y_A, Y_B, Y_CHIP = 220, 120, 160, 480
X_LO, X_HI = 560, 1040                   # U1 bits 3..0 (left), U2 bits 7..4 (right)
LINE_X = {"a": 300, "b": 340, "b2": 820, "a2": 860}

# 7485 pins: 1 B3, 2 A<B in, 3 A=B in, 4 A>B in, 5 A>B out, 6 A=B out, 7 A<B out,
#            9 B0, 10 A0, 11 B1, 12 A1, 13 A2, 14 B2, 15 A3
DATA = {1: ("b", 3), 9: ("b", 0), 10: ("a", 0), 11: ("b", 1), 12: ("a", 1), 13: ("a", 2), 14: ("b", 2), 15: ("a", 3)}

lo = c.ttl("7485", (X_LO, Y_CHIP), label="U1")
hi = c.ttl("7485", (X_HI, Y_CHIP), label="U2")
c.text((X_LO + 80, Y_CHIP - 260), "bits 3..0")
c.text((X_HI + 80, Y_CHIP - 260), "bits 7..4")

taps = {k: [] for k in LINE_X}
sig_lo = dict(DATA)
sig_lo.update({2: ("gnd", None), 3: ("vcc", None), 4: ("gnd", None)})     # alone: A=B in = 1
rails_left(c, lo, sig_lo, LINE_X, taps, {})
rails_left(c, hi, {n: (line + "2", 4 + bit) for n, (line, bit) in DATA.items()}, LINE_X, taps, {})

# ---- cascade: U1 outputs -> U2 cascade inputs, nested under the chips ---------------
for out, inp, depth in ((7, 2, 40), (6, 3, 60), (5, 4, 80)):      # A<B, A=B, A>B
    y = lo[out][1] + depth
    c.wire(lo[out], (lo[out][0], y), (hi[inp][0], y), hi[inp])

# ---- inputs, left: one pair of lines per chip --------------------------------------
c.wire(c.input((X_IN, Y_A), f"{M}_a", 8), (LINE_X["a"], Y_A), (LINE_X["a2"], Y_A))
c.wire(c.input((X_IN, Y_B), f"{M}_b", 8), (LINE_X["b"], Y_B), (LINE_X["b2"], Y_B))
for k, y in (("a", Y_A), ("b", Y_B), ("a2", Y_A), ("b2", Y_B)):
    c.bus((LINE_X[k], y), taps[k], width=8)

# ---- flags, straight down from U2 ----------------------------------------------------
Y_PIN = hi[7][1] + 200
for i, (n, k) in enumerate(((7, "f"), (6, "e"), (5, "s"))):   # rightmost pin turns first
    y, x = hi[n][1] + 40 + 20 * i, hi[7][0] + 200 - 80 * i
    c.wire(hi[n], (hi[n][0], y), (x, y), c.output((x, Y_PIN), f"{M}_{k}", facing="north"))

c.symbol(left=[f"{M}_a", f"{M}_b"], bottom=[f"{M}_s", f"{M}_e", f"{M}_f"])

if __name__ == "__main__":
    bad = p.save(OUT)
    if "--png" in sys.argv:
        import render
        import netlist
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M)
    sys.exit(1 if bad else 0)
