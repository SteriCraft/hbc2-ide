#!/usr/bin/env python3
"""8-bit logic unit -> circuits/alu/logic8.circ

    logic8_and = a and b     U1 (bits 7..4), U4 (bits 3..0)   74HCT08
    logic8_or  = a or b      U2, U5                           74HCT32
    logic8_xor = a xor b     U3, U6                           74HCT86

NOT A isn't here: the result multiplexer (rmux8) puts it on alu_r through an
inverting buffer (74HCT240), which costs no gate.

Layout: three columns, one per function, each with the add_sub8 style: two
chips stacked (bits 7..4 above), every pin leaving left on a nested rail to a
vertical line (a, b, result) with 1-bit taps. a and b reach each column
through tunnels; each result goes up to a tunnel and on to its output pin.

    python logic8.py           # write, check
    python logic8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python logic8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, nested_rails  # noqa: E402

M = "logic8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

Y_HI, Y_LO = 420, 740            # chip rows: bits 7..4, bits 3..0
Y_A, Y_B, Y_R = 200, 240, 160    # tops of the a, b and result lines
X_IN, X_OUT = 220, 1720          # input / output pins


def rails(chip, signals, line_x, taps):
    """Every pin of `signals` (pin -> (line, bit)) leaves left on a nested
    rail and ends on a 1-bit tap of its line."""
    for side in ("top", "bottom"):
        edge_y = chip.loc[1] + (-30 if side == "top" else 30)
        away = -1 if side == "top" else 1
        pins = {n: chip[n] for n in signals if chip[n][1] == edge_y}
        for name, depth in nested_rails(pins, toward="left").items():
            line, bit = signals[name]
            y = edge_y + away * depth
            end = (line_x[line] + 20, y)
            taps[line].append((end, bit))
            c.wire(end, (chip[name][0], y), chip[name])


for col, (chip_type, fn, what) in enumerate((("7408", "and", "a and b"),
                                             ("7432", "or", "a or b"),
                                             ("7486", "xor", "a xor b"))):
    x0 = 340 + 460 * col                       # the a line of this column
    line_x = {"a": x0, "b": x0 + 40, "r": x0 + 80}
    taps = {k: [] for k in line_x}
    for row, (y, base) in enumerate(((Y_HI, 4), (Y_LO, 0))):
        chip = c.ttl(chip_type, (x0 + 160, y), label=f"U{1 + col + 3 * row}")
        c.text((x0 + 380, y), f"{what}, bits {base + 3}..{base}")
        sig = {}
        for g in range(1, 5):                  # gate g handles bit base + g - 1
            sig[f"{g}A"] = ("a", base + g - 1)
            sig[f"{g}B"] = ("b", base + g - 1)
            sig[f"{g}Y"] = ("r", base + g - 1)
        rails(chip, sig, line_x, taps)

    # a and b come in through tunnels on the left, the result leaves on top
    for line, y in (("a", Y_A), ("b", Y_B)):
        c.bus((line_x[line], y), taps[line], width=8)
        if line == "b":
            c.wire((line_x["b"], y), (x0 - 20, y))
        else:
            c.wire((x0, y), (x0 - 20, y))
        c.tunnel((x0 - 20, y), f"{M}_{line}", 8, facing="east")
    c.bus((line_x["r"], Y_R), taps["r"], width=8)
    c.wire((line_x["r"], Y_R), (line_x["r"] + 20, Y_R))
    c.tunnel((line_x["r"] + 20, Y_R), f"{M}_{fn}", 8, facing="west")

# ---- pins ------------------------------------------------------------------
for y, n in ((60, "a"), (100, "b")):
    c.wire(c.input((X_IN, y), f"{M}_{n}", 8), c.tunnel((X_IN + 20, y), f"{M}_{n}", 8))
for y, n in ((60, "and"), (100, "or"), (140, "xor")):
    c.wire(c.tunnel((X_OUT - 20, y), f"{M}_{n}", 8, facing="east"), c.output((X_OUT, y), f"{M}_{n}", 8))

c.symbol(left=[f"{M}_a", f"{M}_b"], right=[f"{M}_and", f"{M}_or", f"{M}_xor"])

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
