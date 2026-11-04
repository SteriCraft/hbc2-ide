#!/usr/bin/env python3
"""Register section -> circuits/registers/regs.circ

The 8 general registers (A B C D I J X Y = 0..7), the write path, the
operand latches, the A / B operand buses, the index selection and the data
bus drivers. Why it is built this way: notes/registers-decisions.md.

Storage: two copies of the register file, 4 x 74HCT670 each (words 0-3 and
4-7, bits 3..0 and 7..4). Every write goes to both copies; copy A is read
port A (bus A), copy B is read port B (bus B). Each copy's banks are chosen
by /RE, straight from the index multiplexers:

    port A (and write) index: 00 R1, 01 R2, 10 R3, 11 control unit
    port B index:             00 R2, 01 R1, 10 R3, 11 off

    74HCT153 half      D0..D3 (by regs_x_sel)       output
    idx bit 0 / 1      the sources' bits 0 / 1       RA (and WA for port A)
    /RE words 0-3      the sources' bit 2            (CU: regs_cu_bank_n_0; B off: 1)
    /RE words 4-7      not bit 2 (from the IR)       (CU: regs_cu_bank_n_1; B off: 1)

The IR supplies bit 2 of R1, R2 and R3 already inverted (regs_r1_2_n ...,
control-requirements.md C8): an inverter here would be on the critical
read path (28 ns), the IR inverts before its latch, during the fetch.

Write: /WE of a bank = clk_wr_n OR (regs_we_n OR /RE of that bank on port A)
(74HCT32), so the write index is port A's index, and a write can't happen
with port A off. The bracket is made first, from control signals that are
stable long before the strobe, so clk_wr_n goes through one gate only.

Latches (74HCT574 model, clocked by clk on every edge, no enable):
result (alu_r -> data bus, regs_res_oe_n), operand A (data bus -> bus A,
regs_a_lat_oe_n), operand B (data bus -> bus B, regs_b_lat_oe_n).
74HCT541: the constant 0000000c (c = regs_const_0) onto bus B, and bus A
onto the data bus (regs_a_bus_oe_n).

    python regs.py           # write, check, and write notes/bom-regs.csv
    python regs.py --test    # ... and run it in Logisim (hbc2 vectors regs, regs_cpu)
    python regs.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "regs"
OUT = ROOT / "circuits" / "registers" / f"{M}.circ"
P = f"{M}_"

p = Project()
chips_lib = p.library(ROOT / "circuits" / "chips" / "74574.circ")
c = p.circuit(M)
COL = {k: 600 + 420 * k for k in range(6)}
ROW = {k: 160 + 280 * k for k in range(5)}
chips = []


def place(u, pins, note, x, y, **kw):
    chips.append(u)
    pin_tunnels(c, u, pins, **kw)
    if note:
        c.text((x - 10, y - 50), note)
    return u


def chip(part, col, row, pins, note=""):
    return place(c.ttl(part, (COL[col], ROW[row]), facing="south"), pins, note, COL[col], ROW[row])


def latch(col, row, pins, note):
    x, y = COL[col] - 60, 300 + 460 * row       # DIP symbol: anchor = pin 1, bottom left
    return place(c.sub(chips_lib, "ttl74574", (x, y)), pins, note, x - 120, y - 260, stagger=20, levels=6)


def bits(prefix, n=8):
    return [f"{P}{prefix}_{k}" for k in range(n)]


R = {f: bits(f, 3) for f in ("r1", "r2", "r3")}
NOT2 = {f: f"{P}{f}_2_n" for f in R}                  # not bit 2, from the IR

# ---- index selection ---------------------------------------------------------------


def mux(col, row, sel, halves, note):
    """74HCT153: S1 S0 = sel (S1 the high bit); halves = [(D0..D3, Y), (D0..D3, Y)]."""
    pins = {"S1": f"{sel}_1", "S0": f"{sel}_0", "n1E": "gnd", "n2E": "gnd"}
    for h, (ds, y) in zip("12", halves):
        pins.update({f"{h}D{k}": d for k, d in enumerate(ds)})
        pins[f"{h}Y"] = y
    chip("74153", col, row, pins, note)


A_SRC = ("r1", "r2", "r3")                            # + the control unit's index (11)
B_SRC = ("r2", "r1", "r3")                            # + off (11)
mux(0, 0, P + "a_sel", [([R[f][k] for f in A_SRC] + [f"{P}cu_idx_{k}"], f"{P}a_idx_{k}") for k in (0, 1)],
    "port A / write index")
mux(0, 1, P + "a_sel", [([R[f][2] for f in A_SRC] + [P + "cu_bank_n_0"], P + "a_re_lo_n"),
                        ([NOT2[f] for f in A_SRC] + [P + "cu_bank_n_1"], P + "a_re_hi_n")],
    "port A banks")
mux(0, 2, P + "b_sel", [([R[f][k] for f in B_SRC] + ["gnd"], f"{P}b_idx_{k}") for k in (0, 1)],
    "port B index")
mux(0, 3, P + "b_sel", [([R[f][2] for f in B_SRC] + ["vcc"], P + "b_re_lo_n"),
                        ([NOT2[f] for f in B_SRC] + ["vcc"], P + "b_re_hi_n")],
    "port B banks (11 = off)")

# ---- the register file: 2 copies x 2 banks x 2 nibbles ---------------------------------
# Logisim's 74670 names TI's D0..D3 / Q0..Q3 as D1..D4 / Q1..Q4; nOE is /RE.
for col, port in ((1, "a"), (2, "b")):
    for row, (bank, nib) in enumerate(((lo, n) for lo in ("lo", "hi") for n in (0, 1))):
        pins = {"WA0": P + "a_idx_0", "WA1": P + "a_idx_1", "RA0": f"{P}{port}_idx_0",
                "RA1": f"{P}{port}_idx_1", "nOE": f"{P}{port}_re_{bank}_n", "nWE": f"{P}we_{bank}_n"}
        for k in range(4):
            pins[f"D{k + 1}"] = f"{P}bus_in_{4 * nib + k}"
            pins[f"Q{k + 1}"] = f"{P}{port}_{4 * nib + k}"
        words = "words 0-3" if bank == "lo" else "words 4-7"
        chip("74670", col, row, pins, f"port {port.upper()}, {words}, bits {4 * nib + 3}..{4 * nib}")

# ---- write enables: 74HCT32 ------------------------------------------------------------
chip("7432", 3, 0, {"1A": P + "we_n", "1B": P + "a_re_lo_n", "1Y": P + "en_lo_n",
                    "2A": "clk_wr_n", "2B": P + "en_lo_n", "2Y": P + "we_lo_n",
                    "3A": P + "we_n", "3B": P + "a_re_hi_n", "3Y": P + "en_hi_n",
                    "4A": "clk_wr_n", "4B": P + "en_hi_n", "4Y": P + "we_hi_n"}, "/WE per bank")

# ---- 74HCT541: constant onto bus B, bus A onto the data bus ----------------------------
chip("74541", 3, 1, {"nOE1": P + "const_oe_n", "nOE2": "gnd", "A1": P + "const_0",
                     **{f"A{k + 1}": "gnd" for k in range(1, 8)},
                     **{f"Y{k + 1}": f"{P}b_{k}" for k in range(8)}}, "constant 0000000c → B")
chip("74541", 3, 2, {"nOE1": P + "a_bus_oe_n", "nOE2": "gnd",
                     **{f"A{k + 1}": f"{P}a_{k}" for k in range(8)},
                     **{f"Y{k + 1}": f"{P}bus_out_{k}" for k in range(8)}}, "bus A → data bus")

# ---- 74HCT574 latches, clocked on every edge ---------------------------------------------
for row, (src, dst, oe, note) in enumerate((("alu_r", "bus_out", "res_oe_n", "result latch → data bus"),
                                            ("bus_in", "a", "a_lat_oe_n", "operand latch → bus A"),
                                            ("bus_in", "b", "b_lat_oe_n", "operand latch → bus B"))):
    latch(4, row, {"/OE": P + oe, "CP": "clk", **{f"D{k}": f"{P}{src}_{k}" for k in range(8)},
                   **{f"Q{k}": f"{P}{dst}_{k}" for k in range(8)}}, note)

# chips numbered top to bottom, then left to right
for i, u in enumerate(sorted(chips, key=lambda u: (u.loc[1] if u.lib == "#TTL" else u.loc[1] - 100, u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ------------------------------------------------------------------------
TOP = [("clk", 1), ("clk_wr_n", 1), (P + "we_n", 1), (P + "a_sel", 2), (P + "cu_idx", 2),
       (P + "cu_bank_n", 2), (P + "b_sel", 2), (P + "a_lat_oe_n", 1), (P + "b_lat_oe_n", 1),
       (P + "const_oe_n", 1), (P + "const_0", 1), (P + "a_bus_oe_n", 1), (P + "res_oe_n", 1)]
LEFT = [(P + "r1", 3), (P + "r2", 3), (P + "r3", 3), (P + "r1_2_n", 1), (P + "r2_2_n", 1), (P + "r3_2_n", 1),
        (P + "alu_r", 8), (P + "bus_in", 8)]
RIGHT = [(P + "a", 8), (P + "b", 8), (P + "bus_out", 8)]
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
    import bom      # the bill of materials follows the circuit (notes/bom-regs.csv)
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-regs.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-regs.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "regs")
        bad += not logitest.check_emulator(OUT, M, "regs_cpu")
    sys.exit(1 if bad else 0)
