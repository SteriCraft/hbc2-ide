#!/usr/bin/env python3
"""Flags of a result -> circuits/alu/flags8.circ

    flags8_z = r is 0          U1 (74HCT27) nor3(r0,r1,r2), nor3(r3,r4,r5), nor3(r6,r7,0)
                               U2 (74HCT11) and3 of the three
    flags8_n = r7

U2 gates 1-2 are unused (inputs to ground). The CMP flags S E F come from
cmp8 (two 74HCT85), not from the result (alu-decisions.md D15).

Layout: r in on the left, the chips stacked, every pin leaving left on a
nested rail to a vertical line; the flags leave at the bottom.

    python flags8.py           # write, check
    python flags8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python flags8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, rails_left  # noqa: E402

M = "flags8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

X_IN, Y_R = 220, 120
LINE_X = {"r": 300, "n1": 380, "n2": 400, "n3": 420, "z": 440}
X_CHIP = 540
taps = {"r": []}
ends = {k: [] for k in ("n1", "n2", "n3", "z")}

nor = c.ttl("7427", (X_CHIP, 400), label="U1")
and3 = c.ttl("7411", (X_CHIP, 720), label="U2")
c.text((X_CHIP + 260, 400), "z: nor of r")
c.text((X_CHIP + 260, 720), "z: and")

g = ("gnd", None)
rails_left(c, nor, {"1A": ("r", 0), "1B": ("r", 1), "1C": ("r", 2), "1Y": ("n1", None),
                    "2A": ("r", 3), "2B": ("r", 4), "2C": ("r", 5), "2Y": ("n2", None),
                    "3A": ("r", 6), "3B": ("r", 7), "3C": g, "3Y": ("n3", None)}, LINE_X, taps, ends)
rails_left(c, and3, {"3A": ("n1", None), "3B": ("n2", None), "3C": ("n3", None), "3Y": ("z", None),
                     "1A": g, "1B": g, "1C": g, "2A": g, "2B": g, "2C": g}, LINE_X, taps, ends)

for k in ("n1", "n2", "n3"):
    c.wire((LINE_X[k], min(ends[k])), (LINE_X[k], max(ends[k])))

# ---- input, left; flags, bottom ------------------------------------------------
Y_PIN = 1020
c.wire(c.input((X_IN, Y_R), f"{M}_r", 8), (LINE_X["r"], Y_R))
taps["r"].append(((LINE_X["r"] + 20, 920), 7))          # n = r7, straight down
c.bus((LINE_X["r"], Y_R), taps["r"], width=8)
c.wire((LINE_X["r"] + 20, 920), c.output((LINE_X["r"] + 20, Y_PIN), f"{M}_n", facing="north"))
x = LINE_X["z"]
c.wire((x, min(ends["z"])), (x, 940), (520, 940), c.output((520, Y_PIN), f"{M}_z", facing="north"))

c.symbol(left=[f"{M}_r"], bottom=[f"{M}_n", f"{M}_z"])

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
