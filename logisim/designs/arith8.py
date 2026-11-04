#!/usr/bin/env python3
"""8-bit arithmetic: ADD ADC SUB SBB INC DEC -> circuits/alu/arith8.circ

add_sub8 (circuits/alu/add_sub8.circ, a library) does the work. In front of
its B input, two 74HCT157 (U1 bits 7..4, U2 bits 3..0) give

    b' = b     when arith8_one = 0
    b' = 01    when arith8_one = 1      (the 157's B inputs tied to 0 / 1)

so INC = a + 01 and DEC = a - 01 through the same adder, with use_c = 0. The
carry out is then exactly the emulator's INC / DEC carry (a = FF / a = 00).

    one sub use_c   s               c
    0   0   0       a + b           carry       ADD
    0   0   1       a + b + cin     carry       ADC
    0   1   0       a - b           borrow      SUB (and CMP)
    0   1   1       a - b - cin     borrow      SBB
    1   0   0       a + 1           carry       INC
    1   1   0       a - 1           borrow      DEC
    1   x   1       a +- 1 +- cin               (unused)

Layout: control on top, a and b on the left, s on the right, c at the bottom.
The 157s use the add_sub8 style (nested rails to the b, one and b' lines);
b' goes over the top into add_sub8's b port.

    python arith8.py           # write, check
    python arith8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python arith8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, rails_left  # noqa: E402

M = "arith8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
add_sub8 = p.library(OUT.parent / "add_sub8.circ")
c = p.circuit(M)

X_IN = 220
LINE_X = {"b": 300, "one": 340, "bx": 380}
X_MUX = 460
Y_HI, Y_LO = 420, 820             # 16-pin chips: 7 rails per edge, 140 px deep
Y_CTRL, Y_B, Y_BX, Y_A = 40, 120, 160, 80

# ---- B or 01: U1 (bits 7..4), U2 (bits 3..0) --------------------------------
taps = {"b": [], "bx": []}
ends = {"one": []}
for label, y, base in (("U1", Y_HI, 4), ("U2", Y_LO, 0)):
    mux = c.ttl("74157", (X_MUX, y), label=label)
    c.text((X_MUX + 260, y), f"b or 01, bits {base + 3}..{base}")
    sig = {"SELECT": ("one", None), "nSTROBE": ("gnd", None)}
    for k in range(1, 5):                  # section k handles bit base + k - 1
        bit = base + k - 1
        sig[f"{k}A"] = ("b", bit)
        sig[f"{k}B"] = ("vcc" if bit == 0 else "gnd", None)
        sig[f"{k}Y"] = ("bx", bit)
    rails_left(c, mux, sig, LINE_X, taps, ends)

# ---- add_sub8 ----------------------------------------------------------------
u = c.sub(add_sub8, "add_sub8", (1300, 460))
port = lambda n: u[f"add_sub8_{n}"]       # noqa: E731

# ---- pins and lines ------------------------------------------------------------
x_one = LINE_X["one"]
c.wire(c.input((x_one, Y_CTRL), f"{M}_one", facing="south"), (x_one, max(ends["one"])))
for n in ("sub", "use_c", "cin"):
    x, y = port(n)
    c.wire(c.input((x, Y_CTRL), f"{M}_{n}", facing="south"), (x, y))

c.wire(c.input((X_IN, Y_B), f"{M}_b", 8), (LINE_X["b"], Y_B))
c.bus((LINE_X["b"], Y_B), taps["b"], width=8)
c.bus((LINE_X["bx"], Y_BX), taps["bx"], width=8)
x_turn = port("b")[0] - 40
c.wire((LINE_X["bx"], Y_BX), (x_turn, Y_BX), (x_turn, port("b")[1]), port("b"))

c.wire(c.input((X_IN, Y_A), f"{M}_a", 8), c.tunnel((X_IN + 20, Y_A), f"{M}_a", 8))
c.wire(c.tunnel((port("a")[0] - 20, port("a")[1]), f"{M}_a", 8, facing="east"), port("a"))

c.wire(port("s"), c.output((port("s")[0] + 200, port("s")[1]), f"{M}_s", 8))
c.wire(port("c"), c.output((port("c")[0], 1000), f"{M}_c", facing="north"))

c.symbol(top=[f"{M}_one", f"{M}_sub", f"{M}_use_c", f"{M}_cin"], left=[f"{M}_a", f"{M}_b"],
         right=[f"{M}_s"], bottom=[f"{M}_c"])

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
