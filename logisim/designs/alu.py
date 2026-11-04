#!/usr/bin/env python3
"""ALU -> circuits/alu/alu.circ

For now the ALU does ADD / ADC / SUB / SBB through add_sub8
(circuits/alu/add_sub8.circ, loaded as a Logisim library) and sets the
C, N and Z flags:

    alu_c = add_sub8_c   (carry, or borrow after SUB/SBB)
    alu_n = s bit 7
    alu_z = nor of the 8 bits of s:
            U1 (74HCT27) nor3(s0, s1, gnd), nor3(s2, s3, s4), nor3(s5, s6, s7)
            U2 (74HCT11) and3 of the three

Layout follows the conventions: control on top, data in on the left, result
on the right, flags at the bottom, in the schematic and in the symbol. U1's
pins leave left on nested rails toward the s bus (1-bit taps) and three
lines carrying its outputs down to U2.

    python alu.py           # write, check
    python alu.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python alu.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, nested_rails  # noqa: E402

M = "alu"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
add_sub8 = p.library(OUT.parent / "add_sub8.circ")
c = p.circuit(M)

u = c.sub(add_sub8, "add_sub8", (500, 300))
port = lambda n: u[f"add_sub8_{n}"]       # noqa: E731

# ---- control, top; data, left -----------------------------------------------
for n in ("sub", "use_c", "cin"):
    x, y = port(n)
    c.wire(c.input((x, 100), f"{M}_{n}", facing="south"), (x, y))
c.wire(c.input((60, port("a")[1]), f"{M}_a", 8), port("a"))
c.wire(c.input((60, port("b")[1]), f"{M}_b", 8), port("b"))

# ---- result, right ----------------------------------------------------------
S_X = 560                                  # the s bus down to the flag logic
c.wire(port("s"), c.output((1060, port("s")[1]), f"{M}_s", 8))

# ---- Z: U1 (74HCT27), U2 (74HCT11) -----------------------------------------
X = 720
nor = c.ttl("7427", (X, 480), label="U1")
and3 = c.ttl("7411", (X, 770), label="U2")
c.text((X + 230, 480), "z: nor of s")
c.text((X + 230, 770), "z: and")

LINE_X = {"s": S_X, "n1": 600, "n2": 620, "n3": 640}
taps = []                                  # (end, bit) on the s bus
line_pts = {k: [] for k in ("n1", "n2", "n3")}


def rails(chip, signals, depth0=20):
    """Every pin of `signals` leaves left on a nested rail: to an s-bus tap
    ("s", bit), to a 1-bit line ("n1".."n3"), or to Ground ("gnd")."""
    for side in ("top", "bottom"):
        edge_y = chip.loc[1] + (-30 if side == "top" else 30)
        away = -1 if side == "top" else 1
        pins = {n: chip[n] for n in signals if chip[n][1] == edge_y}
        for name, depth in nested_rails(pins, toward="left", first=depth0).items():
            line, bit = signals[name]
            y = edge_y + away * depth
            if line == "gnd":
                end = (chip.loc[0] - 10, y)
                c.add("#Wiring", "Ground", end, facing="west")
            elif line == "s":
                end = (S_X + 20, y)
                taps.append((end, bit))
            else:
                end = (LINE_X[line], y)
                line_pts[line].append(y)
            c.wire(end, (chip[name][0], y), chip[name])


rails(nor, {"1A": ("s", 0), "1B": ("s", 1), "1C": ("gnd", None), "1Y": ("n1", None),
            "2A": ("s", 2), "2B": ("s", 3), "2C": ("s", 4), "2Y": ("n2", None),
            "3A": ("s", 5), "3B": ("s", 6), "3C": ("s", 7), "3Y": ("n3", None)})
rails(and3, {"1C": ("gnd", None), "3C": ("n1", None), "3B": ("n2", None), "3A": ("n3", None)})

for line, ys in line_pts.items():          # each line joins U1's output to U2's input
    c.wire((LINE_X[line], min(ys)), (LINE_X[line], max(ys)))

# U2's unused bottom inputs to ground
y = and3["1A"][1] + 10
for n in ("1A", "1B", "2A", "2B", "2C"):
    c.wire(and3[n], (and3[n][0], y))
c.wire((and3["1A"][0], y), (and3["2C"][0], y))
c.add("#Wiring", "Ground", (and3["1A"][0], y))

# ---- flags, bottom ----------------------------------------------------------
Y_FLAGS = 940
c.wire(port("c"), c.output((port("c")[0], Y_FLAGS), f"{M}_c", facing="north"))

y_n = 880                                  # N = s bit 7
taps.append(((S_X + 20, y_n), 7))
c.wire((S_X + 20, y_n), (700, y_n), c.output((700, Y_FLAGS), f"{M}_n", facing="north"))

x_z = 1020
z = and3["3Y"]
c.wire(z, (z[0], z[1] - 10), (x_z, z[1] - 10), c.output((x_z, Y_FLAGS), f"{M}_z", facing="north"))

c.bus((S_X, port("s")[1]), taps, width=8)

c.symbol(top=[f"{M}_sub", f"{M}_use_c", f"{M}_cin"], left=[f"{M}_a", f"{M}_b"],
         right=[f"{M}_s"], bottom=[f"{M}_c", f"{M}_n", f"{M}_z"])

# Gianni moved everything by (160, 30) in Logisim so the labels on the left show.
c.move(160, 30)

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
