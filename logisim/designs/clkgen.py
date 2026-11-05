#!/usr/bin/env python3
"""Clock and reset generator -> circuits/cu/clkgen.circ

control-requirements.md C6 (clock), C17 (reset), C24 (rst_rc for the
pipeline /OE); cu-decisions.md sections 7 and 14.

    clock    osc (4 MHz module) -> 74HCT74 FF1, D = /Q: clk (2 MHz), clk_n
             FF2 clocked by NOT osc (a 74HCT10 gate), D = clk: Q2, /Q2
             clk_wr_n     = NAND3(clk_n, osc, Q2)   (74HCT10)   low 250-375 ns
             clk_mem_wr_n = /Q2                                  low 125-375 ns
    buffer   74HCT541: Y1..Y6 = clk branches (the 6-bit pin clk, about 5 clock
             inputs each), Y7 = clk_wr_n, Y8 = clk_mem_wr_n; all three through it,
             so their edges keep the same delay (section 7 item 1)
    reset    RC node -> 74HCT14 -> rst_rc (high while the RC is low: the
             pipeline 574s' /OE, C24) -> 74HCT14 -> both /CLR of a 74HCT74
             clocked by clk: FF1 D = VCC gives rst_early_n (Q) / rst_early (/Q),
             FF2 D = FF1 Q gives rst_n (asynchronous assert, release just after
             a clk edge, two edges after the RC)

The RC (resistor, capacitor, push button, diode) is analog: clk_rc is the
node at the 74HCT14 input. The oscillator module is clk_osc.

    python clkgen.py           # write, check, and write notes/bom-clkgen.csv
    python clkgen.py --test    # ... and run the waveform sequences in Logisim
    python clkgen.py --png     # ... and draw it next to the .circ
"""

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "clkgen"
OUT = ROOT / "circuits" / "cu" / f"{M}.circ"
P = f"{M}_"
BRANCHES = 6            # clk branches out of the 541 (section 14: ir, addr, cu x 2, regs + flags, reset FFs + IOD)

p = Project()
c = p.circuit(M)
COL = {k: 600 + 420 * k for k in range(3)}
ROW = {k: 160 + 300 * k for k in range(2)}
chips = []


def chip(part, col, row, pins, note=""):
    u = c.ttl(part, (COL[col], ROW[row]), facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    if note:
        c.text((COL[col] - 10, ROW[row] - 50), note)
    return u


def n(name):
    return P + name


# ---- clock: 74HCT74 + 74HCT10 ------------------------------------------------------------------
chip("7474", 0, 0, {"CLK1": n("osc"), "D1": n("clk_n"), "Q1": n("clk_q"), "nQ1": n("clk_n"),
                    "nPRE1": "vcc", "nCLR1": "vcc",
                    "CLK2": n("osc_n"), "D2": n("clk_q"), "Q2": n("q2"), "nQ2": n("q2_n"),
                    "nPRE2": "vcc", "nCLR2": "vcc"},
     "clock: FF1 = clk (osc / 2), FF2 = clk on the osc fall (C6)")
chip("7410", 1, 0, {"1A": n("clk_n"), "1B": n("osc"), "1C": n("q2"), "1Y": n("wr_n"),
                    "2A": n("osc"), "2B": "vcc", "2C": "vcc", "2Y": n("osc_n"),
                    "3A": "gnd", "3B": "gnd", "3C": "gnd"},
     "clk_wr_n = NAND3(clk_n, osc, Q2); NOT osc for FF2")
buf = {"nOE1": "gnd", "nOE2": "gnd",
       "A7": n("wr_n"), "Y7": "clk_wr_n", "A8": n("q2_n"), "Y8": "clk_mem_wr_n"}
for k in range(BRANCHES):
    buf[f"A{k + 1}"] = n("clk_q")
    buf[f"Y{k + 1}"] = f"clk_{k}"
chip("74541", 2, 0, buf, "clock buffer: 6 clk branches, clk_wr_n, clk_mem_wr_n")

# ---- reset: 74HCT14 + 74HCT74 ------------------------------------------------------------------
inv = {"1A": n("rc"), "1Y": "rst_rc", "2A": "rst_rc", "2Y": n("rst_clr_n")}
inv.update({f"{g}A": "gnd" for g in range(3, 7)})
chip("7414", 0, 1, inv, "reset: RC -> rst_rc -> /CLR (C17, C24)")
chip("7474", 1, 1, {"CLK1": "clk_5", "D1": "vcc", "Q1": n("rst_early_n"), "nQ1": "rst_early",
                    "nPRE1": "vcc", "nCLR1": n("rst_clr_n"),
                    "CLK2": "clk_5", "D2": n("rst_early_n"), "Q2": "rst_n",
                    "nPRE2": "vcc", "nCLR2": n("rst_clr_n")},
     "reset synchroniser: FF1 rst_early, FF2 rst_n (C17)")

for i, u in enumerate(sorted(chips, key=lambda u: (u.loc[1], u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ---------------------------------------------------------------------------
LEFT = [(n("osc"), 1), (n("rc"), 1)]
RIGHT = [("clk", BRANCHES), ("clk_wr_n", 1), ("clk_mem_wr_n", 1), ("rst_n", 1), ("rst_early", 1), ("rst_rc", 1)]
X_IN, y = 220, 120
for name, w in LEFT:
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60
X_OUT, y = COL[2] + 400, ROW[0]
for name, w in RIGHT:
    bus_tunnels(c, (X_OUT, y), name, w, output=True)
    y += 60 if w == 1 else 20 * w + 60
c.text((COL[0] - 10, ROW[1] + 200), "clk bits: 0 ir, 1 addr, 2 cu (574s 1-5), 3 cu (574s 6-8, 175s, 377), 4 regs + flags, 5 the reset flip-flops + the IOD (cu-decisions.md section 14)")

c.symbol(left=[nm for nm, _ in LEFT], right=[nm for nm, _ in RIGHT])

INPUTS = dict(LEFT)
OUTPUTS = dict(RIGHT)


# ---- expected waveforms (C6, C17) ---------------------------------------------------------------
def sequences(seed=6, runs=12):
    """Each row is one osc half period (125 ns). Every run starts from
    Logisim's power-up (flip-flops at 0) with the RC low, releases it after a
    random number of half periods, then runs, and presses the button again
    at a random time. Expected values follow C6 and C17 directly."""
    rng = random.Random(seed)
    sets = []
    for _ in range(runs):
        rows = []
        clk_q = q2 = 0
        early_n = rst_n = 0
        osc = 0
        # the RC changes only in rows where osc falls, never with a clk edge
        release = 2 * rng.randrange(2, 7)
        press = release + 2 * rng.randrange(6, 20)
        again = press + 2 * rng.randrange(1, 3)
        for t in range(again + 30):
            rc = 0 if t < release or press <= t < again else 1
            prev_osc, osc = osc, 1 - osc if t else 0
            rst_clr = rc == 0
            clk_prev = clk_q
            if osc and not prev_osc:            # osc rises: FF1 toggles
                clk_q ^= 1
            if prev_osc and not osc:            # osc falls: FF2 samples clk
                q2 = clk_q
            if clk_q and not clk_prev and not rst_clr:  # clk rises: the reset synchroniser
                early_n, rst_n = 1, early_n
            if rst_clr:
                early_n = rst_n = 0
            rows.append({n("osc"): osc, n("rc"): rc,
                         "clk": (1 << BRANCHES) - 1 if clk_q else 0,
                         "clk_wr_n": 0 if (not clk_q and osc and q2) else 1,
                         "clk_mem_wr_n": 1 - q2,
                         "rst_n": rst_n, "rst_early": 1 - early_n, "rst_rc": 1 - rc})
        sets.append(rows)
    return sets


if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-clkgen.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-clkgen.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, M, INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
