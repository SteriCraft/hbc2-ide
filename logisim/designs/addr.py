#!/usr/bin/env python3
"""Address section -> circuits/addr/addr.circ

PC, SP, the 16-bit address adder and the address latch, with their bus
drivers. Why it is built this way: notes/addr-decisions.md.

Everything goes through one adder, sum = BASE + (OFFSET xor inv) + cin:

    BASE   (8 x 74HCT153, addr_base_sel, /E per byte = addr_base_oe_n):
           00 PC   01 SP   10 bus A : bus B (R1:R2)   11 IVT = 1111111 p7 : p6..p0 0
           (p = the low byte of the address bus, read back: addr_bus_addr_lo_in)
    OFFSET (4 x 74HCT157, /G per byte = addr_off_oe_n):
           high byte V1, low byte V2 (addr_off_sel = 0) or V1 (1); off = 00
    inv    (4 x 74HCT86): the offset inverted, for subtraction and -1
    adder  (4 x 74HCT283), carry in addr_cin

The latch, PC and SP (2 x 74HCT377 each, clocked by clk on every edge, load
enable /E) all load the sum. The latch drives the address bus through
2 x 74HCT541 (/OE per byte); PC and SP drive the data bus through
2 x 74HCT541 each (one byte at a time).

Reset (addr-decisions.md A1): while rst_n is low the control unit holds the
reset word (BASE = IVT high byte only, no offset, PC and SP loaded), so the
sum is FE00: SP := FE00. Two 74HCT08 force PC's bits 15..9 and the IVT's
p7 to 0 while rst_n is low, so PC := 0000 on the same edge.

    python addr.py           # write, check, and write notes/bom-addr.csv
    python addr.py --test    # ... and run it in Logisim (hbc2 vectors addr, addr_cpu)
    python addr.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "addr"
OUT = ROOT / "circuits" / "addr" / f"{M}.circ"
P = f"{M}_"

p = Project()
c = p.circuit(M)
COL = {k: 600 + 420 * k for k in range(6)}
ROW = {k: 160 + 280 * k for k in range(6)}
chips = []


def chip(part, col, row, pins, note=""):
    u = c.ttl(part, (COL[col], ROW[row]), facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    if note:
        c.text((COL[col] - 10, ROW[row] - 50), note)
    return u


def n(name, k=None):
    return f"{P}{name}" if k is None else f"{P}{name}_{k}"


# ---- BASE: 8 x 74HCT153, one bit per half ------------------------------------------------


def base_inputs(b):
    """D0..D3 of bit b: PC, SP, R1:R2 (bus A high, bus B low), IVT."""
    reg = n("bus_b", b) if b < 8 else n("bus_a", b - 8)
    if b == 0:
        ivt = "gnd"
    elif b < 8:
        ivt = n("bus_addr_lo_in", b - 1)
    elif b == 8:
        ivt = n("ivt_8")                 # p7, forced to 0 while rst_n is low
    else:
        ivt = "vcc"
    return [n("pc", b), n("sp", b), reg, ivt]


for j in range(8):
    lo = 2 * j                           # bits lo (half 1) and lo + 1 (half 2)
    byte = 0 if j < 4 else 1
    pins = {"S1": n("base_sel", 1), "S0": n("base_sel", 0),
            "n1E": n("base_oe_n", byte), "n2E": n("base_oe_n", byte)}
    for h, b in (("1", lo), ("2", lo + 1)):
        pins.update({f"{h}D{k}": s for k, s in enumerate(base_inputs(b))})
        pins[f"{h}Y"] = n("base", b)
    chip("74153", byte, j % 4, pins, f"BASE bits {lo + 1}..{lo}")

# ---- OFFSET: 4 x 74HCT157 ------------------------------------------------------------------
for j in range(4):
    lo = 4 * j
    pins = {}
    if lo < 8:                           # low byte: V2 (sel 0) or V1 (sel 1)
        pins.update({"SELECT": n("off_sel"), "nSTROBE": n("off_oe_n", 0)})
        for k in range(4):
            pins.update({f"{k + 1}A": n("v2", lo + k), f"{k + 1}B": n("v1", lo + k), f"{k + 1}Y": n("off", lo + k)})
        note = f"OFFSET bits {lo + 3}..{lo}: V2 / V1"
    else:                                # high byte: V1 or 00
        pins.update({"SELECT": "gnd", "nSTROBE": n("off_oe_n", 1)})
        for k in range(4):
            pins.update({f"{k + 1}A": n("v1", lo - 8 + k), f"{k + 1}B": "gnd", f"{k + 1}Y": n("off", lo + k)})
        note = f"OFFSET bits {lo + 3}..{lo}: V1"
    chip("74157", j // 2, 4 + j % 2, pins, note)

# ---- inversion: 4 x 74HCT86 ----------------------------------------------------------------
for j in range(4):
    pins = {}
    for g in range(4):
        b = 4 * j + g
        pins.update({f"{g + 1}A": n("off", b), f"{g + 1}B": n("inv"), f"{g + 1}Y": n("offx", b)})
    chip("7486", 2, j, pins, f"OFFSET xor inv, bits {4 * j + 3}..{4 * j}")

# ---- reset: 2 x 74HCT08 --------------------------------------------------------------------
gates = [(n("sum", b), n("pc_d", b)) for b in range(9, 16)] + [(n("bus_addr_lo_in", 7), n("ivt_8"))]
for j in range(2):
    pins = {}
    for g in range(4):
        a, y = gates[4 * j + g]
        pins.update({f"{g + 1}A": a, f"{g + 1}B": "rst_n", f"{g + 1}Y": y})
    chip("7408", 2, 4 + j, pins, "reset: PC bits 15..12" if j == 0 else "reset: PC 11..9, IVT p7")

# ---- adder: 4 x 74HCT283 -------------------------------------------------------------------
for j in range(4):
    pins = {"CIN": n("cin") if j == 0 else n("c4", j - 1)}
    if j < 3:
        pins["C4"] = n("c4", j)
    for k in range(4):
        b = 4 * j + k
        pins.update({f"A{k + 1}": n("base", b), f"B{k + 1}": n("offx", b), f"∑{k + 1}": n("sum", b)})
    chip("74283", 3, j, pins, f"adder bits {4 * j + 3}..{4 * j}")

# ---- registers: 6 x 74HCT377, drivers: 6 x 74HCT541 ----------------------------------------
REGS = (("lat", "lat_ld_n", "address latch"), ("pc", "pc_ld_n", "PC"), ("sp", "sp_ld_n", "SP"))
for r, (reg, ld, what) in enumerate(REGS):
    for byte in (1, 0):
        row = 2 * r + (1 - byte)
        pins = {"nCLKen": n(ld), "CLK": "clk"}
        for k in range(8):
            b = 8 * byte + k
            d = n("pc_d", b) if reg == "pc" and b >= 9 else n("sum", b)
            pins.update({f"D{k + 1}": d, f"Q{k + 1}": n(reg, b)})
        chip("74377", 4, row, pins, f"{what} {'high' if byte else 'low'} byte")
        if reg == "lat":
            out, oe = n("bus_addr_" + ("hi" if byte else "lo")), n("lat_oe_n", byte)
            note = f"latch {'high' if byte else 'low'} → address bus"
        else:
            out, oe = n("bus_out"), n(f"{reg}_oe_n", byte)
            note = f"{what} {'high' if byte else 'low'} → data bus"
        pins = {"nOE1": oe, "nOE2": "gnd"}
        for k in range(8):
            pins.update({f"A{k + 1}": n(reg, 8 * byte + k), f"Y{k + 1}": f"{out}_{k}"})
        chip("74541", 5, row, pins, note)

# chips numbered top to bottom, then left to right
for i, u in enumerate(sorted(chips, key=lambda u: (u.loc[1], u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ---------------------------------------------------------------------------
TOP = [("clk", 1), ("rst_n", 1), (P + "base_sel", 2), (P + "base_oe_n", 2), (P + "off_sel", 1),
       (P + "off_oe_n", 2), (P + "inv", 1), (P + "cin", 1), (P + "lat_ld_n", 1), (P + "pc_ld_n", 1),
       (P + "sp_ld_n", 1), (P + "lat_oe_n", 2), (P + "pc_oe_n", 2), (P + "sp_oe_n", 2)]
LEFT = [(P + "bus_a", 8), (P + "bus_b", 8), (P + "v1", 8), (P + "v2", 8), (P + "bus_addr_lo_in", 8)]
RIGHT = [(P + "bus_addr_hi", 8), (P + "bus_addr_lo", 8), (P + "bus_out", 8)]
X_IN, y = 220, 120
for name, w in TOP + LEFT:
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = COL[5] + 300, ROW[1]
for name, w in RIGHT:
    bus_tunnels(c, (X_OUT, y), name, w, output=True)
    y += 20 * w + 60

c.symbol(top=[n for n, _ in TOP], left=[n for n, _ in LEFT], right=[n for n, _ in RIGHT])

INPUTS = dict(TOP + LEFT)
OUTPUTS = dict(RIGHT)

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom      # the bill of materials follows the circuit (notes/bom-addr.csv)
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-addr.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-addr.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "addr")
        bad += not logitest.check_emulator(OUT, M, "addr_cpu")
    sys.exit(1 if bad else 0)
