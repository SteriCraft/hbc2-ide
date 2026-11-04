#!/usr/bin/env python3
"""8-bit adder from two 74283 (4-bit adders) -> circuits/alu/add8.circ

    add8_s, add8_cout = add8_a + add8_b + add8_cin

Layout: U1 (bits 7..4) above U2 (bits 3..0), so the carry between them is a
short jog. Every chip pin leaves toward the left on a nested rail, where it
meets the A, B or S bus through a 1-bit tap. Bit wires never cross each
other; they only cross the bus lines.

    python add8.py           # write, check
    python add8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python add8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, nested_rails  # noqa: E402

OUT = ROOT / "circuits" / "alu" / "add8.circ"     # practice module, kept next to alu

p = Project()
c = p.circuit("add8")

X = 360                                  # left edge of both chips
hi = c.ttl("74283", (X, 300), label="U1")
lo = c.ttl("74283", (X, 620), label="U2")
c.text((X + 230, 300), "bits 7..4")
c.text((X + 230, 620), "bits 3..0")

BUS_X = {"B": 200, "A": 240, "S": 280}   # vertical buses, left of the chips
BUS_TOP = {"S": 50, "A": 80, "B": 120}   # where each bus starts
taps = {b: [] for b in BUS_X}

# chip pin -> (bus, bit offset in the chip's nibble)
SIGNALS = {f"{k}{n}": (k if k != "∑" else "S", n - 1) for k in "AB∑" for n in range(1, 5)}

for chip, base in ((hi, 4), (lo, 0)):
    for side in ("top", "bottom"):
        edge_y = chip.loc[1] + (-30 if side == "top" else 30)
        away = -1 if side == "top" else 1
        pins = {n: chip[n] for n in SIGNALS if chip[n][1] == edge_y}
        for name, depth in nested_rails(pins, toward="left").items():
            bus, bit = SIGNALS[name]
            y = edge_y + away * depth
            end = (BUS_X[bus] + 20, y)
            taps[bus].append((end, base + bit))
            c.wire(end, (chip[name][0], y), chip[name])

for bus, x in BUS_X.items():
    c.bus((x, BUS_TOP[bus]), taps[bus], width=8)

# pins
c.wire(c.input((150, BUS_TOP["A"]), "add8_a", 8), (BUS_X["A"], BUS_TOP["A"]))
c.wire(c.input((150, BUS_TOP["B"]), "add8_b", 8), (BUS_X["B"], BUS_TOP["B"]))
c.wire((BUS_X["S"], BUS_TOP["S"]), c.output((600, BUS_TOP["S"]), "add8_s", 8))

# carry in: one rail below U1's six rails
y = lo["CIN"][1] + 140
c.wire(c.input((150, y), "add8_cin"), (lo["CIN"][0], y), lo["CIN"])

# carry U1 -> U2: up the right side, jog just under U2
y = hi["CIN"][1] + 10
c.wire(lo["C4"], (lo["C4"][0], y), (hi["CIN"][0], y), hi["CIN"])

# carry out
y = hi["C4"][1] - 30
c.wire(hi["C4"], (hi["C4"][0], y), c.output((600, y), "add8_cout"))

if __name__ == "__main__":
    bad = p.save(OUT)
    if "--png" in sys.argv:
        import render
        import netlist
        print(render.render(netlist.Project(OUT).circuits["add8"], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        ok = logitest.check_emulator(OUT, "add8")
        bad += not ok
    sys.exit(1 if bad else 0)
