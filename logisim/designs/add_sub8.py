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
from circgen import Project, nested_rails, rails_left  # noqa: E402

M = "add_sub8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

Y_HI, Y_LO, Y_CARRY = 360, 680, 960     # chip rows
X_XOR, X_ADD, X_ADD2, X_MUX = 320, 760, 1360, 1960   # left edges of the blocks


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
xor_lo = c.ttl("7486", (X_XOR, Y_LO), label="U5")
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

# ---- adders: U6 (bits 3..0), U2 (bits 7..4, carry in 0), U3 (bits 7..4, carry in 1)
add_hi = c.ttl("74283", (X_ADD, Y_HI), label="U2")
add_lo = c.ttl("74283", (X_ADD, Y_LO), label="U6")
add_hi1 = c.ttl("74283", (X_ADD2, Y_HI), label="U3")
c.text((X_ADD + 230, Y_HI), "bits 7..4, cin 0")
c.text((X_ADD2 + 230, Y_HI), "bits 7..4, cin 1")
c.text((X_ADD + 230, Y_LO), "bits 3..0")

ADD_X = {"bx": 600, "a": 640, "s": 680}
add_taps = {k: [] for k in ADD_X}
sig = {f"{k}{n}": ({"A": "a", "B": "bx", "∑": "s"}[k], n - 1) for k in "AB∑" for n in range(1, 5)}
for chip, base in ((add_hi, 4), (add_lo, 0)):
    rails(chip, {n: (line, base + bit) for n, (line, bit) in sig.items()}, ADD_X, add_taps, {})
ADD2_X = {"bx2": 1200, "a2": 1240, "s1": 1280}
add2_taps = {k: [] for k in ADD2_X}
rails(add_hi1, {n: ({"bx": "bx2", "a": "a2", "s": "s1"}[line], 4 + bit) for n, (line, bit) in sig.items()},
      ADD2_X, add2_taps, {})

# the carries: U6 C4 picks the high half; U2 / U3 C4 are the two candidates
def carry_tunnel(pin, name):
    """C4 (top edge) up 20, right 20, to a tunnel."""
    x, y = pin
    c.wire(pin, (x, y - 20), (x + 20, y - 20))
    c.tunnel((x + 20, y - 20), f"{M}_{name}", facing="west")


carry_tunnel(add_lo["C4"], "c_lo")
carry_tunnel(add_hi["C4"], "c_hi0")
carry_tunnel(add_hi1["C4"], "c_hi1")
for chip, kind in ((add_hi, "Ground"), (add_hi1, "Power")):     # carry in 0 / 1
    x, y = chip["CIN"]
    c.wire((x, y), (x, y + 20), (x + 20, y + 20))
    c.add("#Wiring", kind, (x + 20, y + 20), facing="east")

# ---- carry select: U4 (sum bits 7..4), U7 (carry out); SELECT = U6 C4 --------
MUX_X = {"s0": 1720, "s1": 1760, "s": 1800}
mux_taps = {k: [] for k in MUX_X}
mux_s = c.ttl("74157", (X_MUX, Y_HI), label="U4")
mux_c = c.ttl("74157", (X_MUX, Y_LO), label="U7")
c.text((X_MUX + 260, Y_HI), "s 7..4: cin 0 or 1")
c.text((X_MUX + 260, Y_LO), "carry out: cin 0 or 1")
sig = {"SELECT": ("tunnel", f"{M}_c_lo"), "nSTROBE": ("gnd", None)}
for k in range(1, 5):
    sig.update({f"{k}A": ("s0", 3 + k), f"{k}B": ("s1", 3 + k), f"{k}Y": ("s", 3 + k)})
rails_left(c, mux_s, sig, MUX_X, mux_taps, {})
sig = {"SELECT": ("tunnel", f"{M}_c_lo"), "nSTROBE": ("gnd", None),
       "1A": ("tunnel", f"{M}_c_hi0"), "1B": ("tunnel", f"{M}_c_hi1"), "1Y": ("tunnel", f"{M}_c4")}
sig.update({f"{k}{s}": ("gnd", None) for k in (2, 3, 4) for s in "AB"})
bottom = {n for n in sig if mux_c[n][1] > Y_LO}       # tunnels: 30 px apart so the boxes don't touch
rails_left(c, mux_c, {n: v for n, v in sig.items() if n not in bottom}, MUX_X, mux_taps, {})
rails_left(c, mux_c, {n: v for n, v in sig.items() if n in bottom}, MUX_X, mux_taps, {}, pitch=30)

# s bits 3..0 come straight from U6: s0 tap -> s tap, under U7 (they cross the
# s line between two of its taps)
for k in range(4):
    y = 920 + 20 * k
    c.wire((MUX_X["s0"] + 20, y), (MUX_X["s"] + 20, y))
    mux_taps["s0"].append(((MUX_X["s0"] + 20, y), k))
    mux_taps["s"].append(((MUX_X["s"] + 20, y), k))

# ---- buses and data pins ---------------------------------------------------
Y_A, Y_S, Y_B, Y_BX = 130, 150, 170, 190
c.bus((XOR_X["b"], Y_B), xor_taps["b"], width=8)
c.bus((XOR_X["bx"], Y_BX), xor_taps["bx"], width=8)
Y_S1, Y_OUT = 170, 110
c.wire((XOR_X["bx"], Y_BX), (ADD_X["bx"], Y_BX), (ADD2_X["bx2"], Y_BX))   # b' over the top to the adders
c.bus((ADD_X["bx"], Y_BX), add_taps["bx"], width=8)
c.bus((ADD_X["a"], Y_A), add_taps["a"], width=8)
c.bus((ADD_X["s"], Y_S), add_taps["s"], width=8)          # s0: U6 bits 3..0, U2 bits 7..4
c.bus((ADD2_X["bx2"], Y_BX), add2_taps["bx2"], width=8)
c.bus((ADD2_X["a2"], Y_A), add2_taps["a2"], width=8)
c.bus((ADD2_X["s1"], Y_S1), add2_taps["s1"], width=8)     # s1: U3 bits 7..4
for k, y in (("s", Y_OUT), ("s0", Y_S), ("s1", Y_S1)):
    c.bus((MUX_X[k], y), mux_taps[k], width=8)

c.wire(c.input((60, Y_A), f"{M}_a", 8), (ADD_X["a"], Y_A), (ADD2_X["a2"], Y_A))
c.wire(c.input((60, Y_B), f"{M}_b", 8), (XOR_X["b"], Y_B))
c.wire((ADD_X["s"], Y_S), (MUX_X["s0"], Y_S))
c.wire((ADD2_X["s1"], Y_S1), (MUX_X["s1"], Y_S1))
c.wire((MUX_X["s"], Y_OUT), c.output((2400, Y_OUT), f"{M}_s", 8))

# ---- control pins, top -----------------------------------------------------
Y_CTRL = 60
Y_SUB = Y_CARRY - 110                   # sub's run to the carry logic, under every rail
c.wire(c.input((140, Y_CTRL), f"{M}_sub"), (XOR_X["sub"], Y_CTRL), (XOR_X["sub"], Y_SUB))
for x, name in ((400, "use_c"), (680, "cin")):
    c.wire(c.input((x, Y_CTRL), f"{M}_{name}"), c.tunnel((x + 20, Y_CTRL), f"{M}_{name}"))

# ---- carry logic: U8 (use_c and cin), U9 (adder carry in, c flag) ----------
and_ = c.ttl("7408", (500, Y_CARRY), label="U8")
xor_c = c.ttl("7486", (add_lo["CIN"][0] - 70, Y_CARRY), label="U9")   # 4Y right under CIN
c.text((730, Y_CARRY), "use_c and cin")
c.text((1030, Y_CARRY), "carry in, c flag")

# U8 gate 4: use_c and cin, inputs from tunnels on the left
TX = 460
c.wire(and_["4B"], (and_["4B"][0], and_["4B"][1] - 30), c.tunnel((TX, and_["4B"][1] - 30), f"{M}_use_c", facing="east"))
c.wire(and_["4A"], (and_["4A"][0], and_["4A"][1] - 60), c.tunnel((TX, and_["4A"][1] - 60), f"{M}_cin", facing="east"))
y = and_["4Y"][1] - 50
c.wire(and_["4Y"], (and_["4Y"][0], y), (xor_c["4B"][0], y), xor_c["4B"])

# U9 gate 4: sub xor (use_c and cin) -> adder carry in
c.wire(xor_c["4Y"], add_lo["CIN"])
c.wire((XOR_X["sub"], Y_SUB), (xor_c["3B"][0], Y_SUB))
c.wire((xor_c["4A"][0], Y_SUB), xor_c["4A"])
c.wire((xor_c["3B"][0], Y_SUB), xor_c["3B"])

# U9 gate 3: carry out (from U7) xor sub -> c flag, bottom
X_COUT = 1080
y = xor_c["3A"][1] - 30
c.wire(c.tunnel((X_COUT, y), f"{M}_c4", facing="west"), (xor_c["3A"][0], y), xor_c["3A"])
c.wire(xor_c["3Y"], (xor_c["3Y"][0], xor_c["3Y"][1] - 10), (1100, xor_c["3Y"][1] - 10),
       c.output((1100, 1060), f"{M}_c", facing="north"))

# unused inputs to ground: U8 gates 1, 2, 3 and U9 gates 1, 2
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

# Gianni moved everything by (210, 90) in Logisim so the labels on the left show.
c.move(210, 90)

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
