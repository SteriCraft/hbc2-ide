#!/usr/bin/env python3
"""8-bit adder / subtractor -> circuits/alu/add_sub8.circ

Two 74283 add; subtraction uses A - B = A + not B + 1:

    b' = b xor sub                       U1, U3 (74HCT86), bits 7..4 / 3..0
    adder carry in = sub xor (use_c and cin)    U5 (74HCT08), U6 (74HCT86)
    add_sub8_c     = carry out xor sub          U6

    sub use_c   ADD  0 0   s = a + b              c = carry
                ADC  0 1   s = a + b + cin        c = carry
                SUB  1 0   s = a - b              c = borrow
                SBB  1 1   s = a - b - cin        c = borrow (cin = borrow in)

Layout:
- Left block: U1/U3 invert B. Every pin leaves left on a nested rail toward
  three vertical lines: b (in), sub, and b' (out).
- Right block: the add8 layout (U2/U4); b' comes over the top as their B bus.
- Bottom row: the carry logic. use_c and cin come from the top pins through
  tunnels. sub continues down its line. The carry out comes down the right side.

    python add_sub8.py           # write, check
    python add_sub8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python add_sub8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, nested_rails  # noqa: E402

M = "add_sub8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

Y_HI, Y_LO, Y_CARRY = 360, 680, 960     # chip rows
X_XOR, X_ADD = 320, 760                 # left edges of the two blocks


def ground(at, facing="south"):
    c.add("#Wiring", "Ground", at, **({} if facing == "south" else {"facing": facing}))


def rails(chip, signals, bus_x, taps, line_ends):
    """Every pin of `signals` leaves left on a nested rail.
    signals: pin -> (line, bit). Bus lines get a 1-bit tap, 1-bit lines
    (in line_ends) are met directly."""
    for side in ("top", "bottom"):
        edge_y = chip.loc[1] + (-30 if side == "top" else 30)
        away = -1 if side == "top" else 1
        pins = {n: chip[n] for n in signals if chip[n][1] == edge_y}
        for name, depth in nested_rails(pins, toward="left").items():
            line, bit = signals[name]
            y = edge_y + away * depth
            if line in line_ends:
                end = (bus_x[line], y)
                line_ends[line].append(y)
            else:
                end = (bus_x[line] + 20, y)
                taps[line].append((end, bit))
            c.wire(end, (chip[name][0], y), chip[name])


# ---- B inversion: U1 (bits 7..4), U3 (bits 3..0) --------------------------
xor_hi = c.ttl("7486", (X_XOR, Y_HI), label="U1")
xor_lo = c.ttl("7486", (X_XOR, Y_LO), label="U3")
c.text((X_XOR + 210, Y_HI), "b 7..4 xor sub")
c.text((X_XOR + 210, Y_LO), "b 3..0 xor sub")

XOR_X = {"b": 160, "sub": 200, "bx": 240}
xor_taps = {"b": [], "bx": []}
sub_ys = {"sub": []}
for chip, base in ((xor_hi, 4), (xor_lo, 0)):
    sig = {}
    for g in range(1, 5):               # gate g handles bit base + g - 1
        sig[f"{g}A"] = ("b", base + g - 1)
        sig[f"{g}B"] = ("sub", None)
        sig[f"{g}Y"] = ("bx", base + g - 1)
    rails(chip, sig, XOR_X, xor_taps, sub_ys)

# ---- adder: U2 (bits 7..4), U4 (bits 3..0) ---------------------------------
add_hi = c.ttl("74283", (X_ADD, Y_HI), label="U2")
add_lo = c.ttl("74283", (X_ADD, Y_LO), label="U4")
c.text((X_ADD + 230, Y_HI), "bits 7..4")
c.text((X_ADD + 230, Y_LO), "bits 3..0")

ADD_X = {"bx": 600, "a": 640, "s": 680}
add_taps = {k: [] for k in ADD_X}
sig = {f"{k}{n}": ({"A": "a", "B": "bx", "∑": "s"}[k], n - 1) for k in "AB∑" for n in range(1, 5)}
for chip, base in ((add_hi, 4), (add_lo, 0)):
    rails(chip, {n: (line, base + bit) for n, (line, bit) in sig.items()}, ADD_X, add_taps, {})

# ---- buses and data pins ---------------------------------------------------
Y_A, Y_S, Y_B, Y_BX = 130, 150, 170, 190
c.bus((XOR_X["b"], Y_B), xor_taps["b"], width=8)
c.bus((XOR_X["bx"], Y_BX), xor_taps["bx"], width=8)
c.wire((XOR_X["bx"], Y_BX), (ADD_X["bx"], Y_BX))          # b' over the top to the adder
c.bus((ADD_X["bx"], Y_BX), add_taps["bx"], width=8)
c.bus((ADD_X["a"], Y_A), add_taps["a"], width=8)
c.bus((ADD_X["s"], Y_S), add_taps["s"], width=8)

c.wire(c.input((60, Y_A), f"{M}_a", 8), (ADD_X["a"], Y_A))
c.wire(c.input((60, Y_B), f"{M}_b", 8), (XOR_X["b"], Y_B))
c.wire((ADD_X["s"], Y_S), c.output((1180, Y_S), f"{M}_s", 8))

# ---- control pins, top -----------------------------------------------------
Y_CTRL = 60
Y_SUB = Y_CARRY - 110                   # sub's run to the carry logic, under every rail
c.wire(c.input((140, Y_CTRL), f"{M}_sub"), (XOR_X["sub"], Y_CTRL), (XOR_X["sub"], Y_SUB))
for x, name in ((400, "use_c"), (680, "cin")):
    c.wire(c.input((x, Y_CTRL), f"{M}_{name}"), c.tunnel((x + 20, Y_CTRL), f"{M}_{name}"))

# ---- carries between the adders --------------------------------------------
y = add_hi["CIN"][1] + 10
c.wire(add_lo["C4"], (add_lo["C4"][0], y), (add_hi["CIN"][0], y), add_hi["CIN"])

# ---- carry logic: U5 (use_c and cin), U6 (adder carry in, c flag) ----------
and_ = c.ttl("7408", (500, Y_CARRY), label="U5")
xor_c = c.ttl("7486", (add_lo["CIN"][0] - 70, Y_CARRY), label="U6")   # 4Y right under CIN
c.text((730, Y_CARRY), "use_c and cin")
c.text((1030, Y_CARRY), "carry in, c flag")

# U5 gate 4: use_c and cin, inputs from tunnels on the left
TX = 460
c.wire(and_["4B"], (and_["4B"][0], and_["4B"][1] - 30), c.tunnel((TX, and_["4B"][1] - 30), f"{M}_use_c", facing="east"))
c.wire(and_["4A"], (and_["4A"][0], and_["4A"][1] - 60), c.tunnel((TX, and_["4A"][1] - 60), f"{M}_cin", facing="east"))
y = and_["4Y"][1] - 50
c.wire(and_["4Y"], (and_["4Y"][0], y), (xor_c["4B"][0], y), xor_c["4B"])

# U6 gate 4: sub xor (use_c and cin) -> adder carry in
c.wire(xor_c["4Y"], add_lo["CIN"])
c.wire((XOR_X["sub"], Y_SUB), (xor_c["3B"][0], Y_SUB))
c.wire((xor_c["4A"][0], Y_SUB), xor_c["4A"])
c.wire((xor_c["3B"][0], Y_SUB), xor_c["3B"])

# U6 gate 3: carry out xor sub -> c flag, bottom
X_COUT = 1080
y = xor_c["3A"][1] - 30
c.wire(add_hi["C4"], (add_hi["C4"][0], add_hi["C4"][1] - 10), (X_COUT, add_hi["C4"][1] - 10),
       (X_COUT, y), (xor_c["3A"][0], y), xor_c["3A"])
c.wire(xor_c["3Y"], (xor_c["3Y"][0], xor_c["3Y"][1] - 10), (1100, xor_c["3Y"][1] - 10),
       c.output((1100, 1060), f"{M}_c", facing="north"))

# unused inputs to ground: U5 gates 1, 2, 3 and U6 gates 1, 2
for chip, top_free in ((and_, ("3B", "3A")), (xor_c, ())):
    if top_free:
        a, b = (chip[n] for n in top_free)
        c.wire(a, (a[0], a[1] - 10), (b[0], b[1] - 10), b)
        ground((b[0], b[1] - 10), facing="north")
    y = chip["1A"][1] + 10
    for n in ("1A", "1B", "2A", "2B"):
        c.wire(chip[n], (chip[n][0], y))
    c.wire((chip["1A"][0], y), (chip["2B"][0], y))
    ground((chip["1A"][0], y))

c.symbol(top=[f"{M}_sub", f"{M}_use_c", f"{M}_cin"], left=[f"{M}_a", f"{M}_b"],
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
