#!/usr/bin/env python3
"""Result multiplexer -> circuits/alu/rmux8.circ

Each source drives the result bus rmux8_r through its own tri-state buffer;
exactly one buffer is on for every alu_op (op3 op2 op1 op0):

    op       source     buffer              enable
    0xxx     sum        74HCT541            op3 itself (active low)
    1000     and        74HCT541            U1 nY0
    1001     or         74HCT541            U1 nY1
    1010     xor        74HCT541            U1 nY2
    1011     not a      74HCT240 (inverts)  U1 nY3
    1100     shl        74HCT541            U1 nY4
    1101     sr         74HCT541            U2 and3(nY5, nY6, nY7): 1101 is an alias of ASR
    1110     sr                             (the fill bit comes from shift8)
    1111     sr

U1 = 74HCT138 (C B A = op2 op1 op0, G1 = op3), U2 = 74HCT11 (gates 2-3 unused).

Layout: inputs come in on the left through tunnels; the buffers stand in a
row under the result bus. A 541 has its inputs on one edge and its outputs on
the other, so one 8-bit splitter feeds it from below and one takes its
outputs up to the bus. The 240 mixes inputs and outputs on each edge: it uses
the rail style, with local a and r lines.

    python rmux8.py           # write, check
    python rmux8.py --test    # ... and run it in Logisim (vectors from hbc2 vectors rmux8)
    python rmux8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, rails_left  # noqa: E402

M = "rmux8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

Y, Y_R = 800, 520               # chip row, result bus
X_IN, X_OUT = 300, 3700
REV = [[7 - k] for k in range(8)]   # vertical splitters: end 0 is the rightmost

# ---- input pins, left --------------------------------------------------------------
c.wire(c.input((X_IN, 100), f"{M}_op", 4), c.tunnel((X_IN + 20, 100), f"{M}_op", 4))
for i, n in enumerate(("sum", "and", "or", "xor", "a", "shl", "sr")):
    y = 140 + 40 * i
    c.wire(c.input((X_IN, y), f"{M}_{n}", 8), c.tunnel((X_IN + 20, y), f"{M}_{n}", 8))

# ---- U1: decoder ----------------------------------------------------------------------
dec = c.ttl("74138", (600, Y), label="U1")
X_OP = 420
taps = {"op": []}
oe = {0: "and", 1: "or", 2: "xor", 3: "not", 4: "shl", 5: "y5", 6: "y6", 7: "y7"}
sig = {"A": ("op", 0), "B": ("op", 1), "C": ("op", 2), "G1": ("op", 3),
       "nG2A": ("gnd", None), "nG2B": ("gnd", None)}
for k, n in oe.items():
    sig[f"nY{k}"] = ("tunnel", f"{M}_{n}_oe_n" if k < 5 else f"{M}_{n}_n")
rails_left(c, dec, sig, {"op": X_OP}, taps, {}, pitch=30)
# op3 is also the sum buffer's enable
y_sum = Y + 300
taps["op"].append(((X_OP + 20, y_sum), 3))
c.wire((X_OP + 20, y_sum), c.tunnel((X_OP + 60, y_sum), f"{M}_sum_oe_n"))
c.wire(c.tunnel((X_OP - 20, Y + 40), f"{M}_op", 4, facing="east"), (X_OP, Y + 40))
c.bus((X_OP, Y + 40), taps["op"], width=4)
c.text((680, Y + 360), "U1: op -> enables")

# ---- U2: shift right enable ------------------------------------------------------------
and3 = c.ttl("7411", (1000, Y), label="U2")
g = ("gnd", None)
rails_left(c, and3, {"1A": ("tunnel", f"{M}_y5_n"), "1B": ("tunnel", f"{M}_y6_n"),
                     "1C": ("tunnel", f"{M}_y7_n"), "1Y": ("tunnel", f"{M}_sr_oe_n"),
                     "2A": g, "2B": g, "2C": g, "3A": g, "3B": g, "3C": g}, {}, {}, {}, pitch=30)
c.text((1070, Y + 360), "U2: sr enable")

# ---- U3: NOT a (74HCT240) -----------------------------------------------------------------
inv = c.ttl("74240", (1500, Y), label="U3")
LINE_X = {"a": 1240, "r": 1300}
taps = {"a": [], "r": []}
sig = {"n1G": ("tunnel", f"{M}_not_oe_n"), "n2G": ("tunnel", f"{M}_not_oe_n")}
for k in range(1, 5):
    sig[f"1A{k}"], sig[f"n1Y{k}"] = ("a", k - 1), ("r", k - 1)
    sig[f"2A{k}"], sig[f"n2Y{k}"] = ("a", k + 3), ("r", k + 3)
rails_left(c, inv, sig, LINE_X, taps, {})
c.bus((LINE_X["a"], Y_R + 40), taps["a"], width=8)
c.wire(c.tunnel((LINE_X["a"] - 20, Y_R + 40), f"{M}_a", 8, facing="east"), (LINE_X["a"], Y_R + 40))
c.bus((LINE_X["r"], Y_R), taps["r"], width=8)
c.text((1600, Y + 300), "U3: not a")

# ---- U4..U9: 74HCT541 buffers ---------------------------------------------------------------
bus_pts = [(LINE_X["r"], Y_R)]
for i, n in enumerate(("sum", "and", "or", "xor", "shl", "sr")):
    x0 = 1800 + 300 * i
    buf = c.ttl("74541", (x0, Y), label=f"U{4 + i}")
    # outputs up to the result bus
    top = c.splitter((x0 + 40, Y_R), 8, REV, facing="south", appear="left", spacing=2)
    for k in range(8):
        c.wire(top[f"end{7 - k}"], buf[f"Y{k + 1}"])
    bus_pts.append((x0 + 40, Y_R))
    # inputs from below
    bot = c.splitter((x0 + 20, Y + 120), 8, REV, facing="north", appear="right", spacing=2)
    for k in range(8):
        c.wire(bot[f"end{7 - k}"], buf[f"A{k + 1}"])
    c.wire((x0 + 20, Y + 120), (x0 + 20, Y + 160), c.tunnel((x0 - 30, Y + 160), f"{M}_{n}", 8, facing="east"))
    # nOE1 = this source's enable, nOE2 tied low
    c.wire(buf["nOE1"], (x0 + 10, Y + 60), c.tunnel((x0 - 30, Y + 60), f"{M}_{n}_oe_n", facing="east"))
    c.wire(buf["nOE2"], (x0 + 30, Y - 50), (x0 - 10, Y - 50))
    c.add("#Wiring", "Ground", (x0 - 10, Y - 50), facing="west")
    c.text((x0 + 100, Y + 220), f"U{4 + i}: {n}")

c.wire(*bus_pts, c.output((X_OUT, Y_R), f"{M}_r", 8))

c.symbol(top=[f"{M}_op"], left=[f"{M}_sum", f"{M}_and", f"{M}_or", f"{M}_xor", f"{M}_a",
                                  f"{M}_shl", f"{M}_sr"], right=[f"{M}_r"])

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
