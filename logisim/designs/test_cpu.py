#!/usr/bin/env python3
"""Trace bench -> circuits/cpu/test_cpu.circ (circuit test_cpu)

The whole CPU (cpu.circ `main`) running on its own clock, for
tools/cputest.py: Logisim runs it headless (`-tty table`) and prints the
bench's output pins after every oscillator half period; cputest turns the
rows into a trace CSV and compares it with the emulator's (`hbc2 compare`).

    osc     a Logisim Clock (one tick = one oscillator half period, 125 ns)
    rc      the reset RC: low from power-up, high after 15 oscillator periods
            (a 74161 and a 74HCT74 here, an RC on the board)
    IOD     the stub (iod-requirements.md; the real IOD is out of scope):
            - port latches: a 256-byte RAM; OUT (IE = 1, RW = 0) writes
              bus_data at bus_addr_lo while clk_wr_n is low (captured at its
              rising edge, I4); IN (IE = 1, RW = 1) drives the latch onto
              bus_data. Power-up 00, as the emulator's IOD without
              peripherals.
            - one hardware interrupt source, armed by the program: OUT to a
              port F0-FF with a value v > 0 raises INT v steps later, for
              port p = the low 4 bits; the vector step (the step after INR)
              drives 00:p (only the low byte: the CPU reads p from it) and
              the port latch of p. INT comes from a flip-flop on clk_5, is
              cleared when INR is seen, and may rise again from the step after
              the vector step (I5, I6). v = 0 disarms.

The output pins are main's (connector, buses, test points) plus the stub's
int and vec, in one column (Logisim prints them top to bottom).

    python test_cpu.py      # write, check
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "designs"))
from circgen import Project, pin_tunnels  # noqa: E402

NAME = "test_cpu"
OUT = ROOT / "circuits" / "cpu" / f"{NAME}.circ"

p = Project()
t = p.circuit(NAME)
cpu = p.library(ROOT / "cpu.circ")
import cpu as cpu_design  # noqa: E402  (main's pins)

u = t.sub(cpu, "main", (1400, 300))
for name, w in cpu_design.INPUTS + cpu_design.OUTPUTS:
    x, y = u[name]
    d = -1 if (name, w) in cpu_design.INPUTS else 1
    t.wire((x, y), (x + 20 * d, y))
    t.tunnel((x + 20 * d, y), name, w, facing="east" if d < 0 else "west")

# ---- clock and reset ----------------------------------------------------------------------------
t.add("#Wiring", "Clock", (100, 100))
t.wire((100, 100), (120, 100))
t.tunnel((120, 100), "osc", facing="west")
rc_cnt = t.ttl("74161", (300, 400), facing="south", label="U1")
pin_tunnels(t, rc_cnt, {"MR/CLR": "vcc", "CP/CLK": "osc", "D0/A": "gnd", "D1/B": "gnd", "D2/C": "gnd",
                        "D3/D": "gnd", "CE/ENP": "vcc", "PE/LOAD": "vcc", "CET/ENT": "vcc", "TC/RC0": "rc_tc"})
rc_ff = t.ttl("7474", (300, 700), facing="south", label="U2")
pin_tunnels(t, rc_ff, {"nCLR1": "vcc", "D1": "vcc", "CLK1": "rc_tc", "nPRE1": "vcc", "Q1": "rc",
                       "nCLR2": "vcc", "D2": "gnd", "CLK2": "gnd", "nPRE2": "vcc"})
t.text((260, 300), "reset RC: rc rises after 15 osc periods")

# clk (6) -> clk_5; bus_addr (16) -> lo (8) -> lo4 / lo bits 4..7
X, Y = 300, 1000
sp = t.splitter((X, Y), 6, [[5]], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "clk", 6, facing="east")
e = sp["end0"]
t.wire(e, (e[0] + 20, e[1]))
t.tunnel((e[0] + 20, e[1]), "clk_5", facing="west")
X, Y = 300, 1300
sp = t.splitter((X, Y), 16, [list(range(8))], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "bus_addr", 16, facing="east")
e = sp["end0"]
t.wire(e, (e[0] + 20, e[1]))
t.tunnel((e[0] + 20, e[1]), "lo", 8, facing="west")
X, Y = 300, 1500
sp = t.splitter((X, Y), 8, [[0, 1, 2, 3], [4], [5], [6], [7]], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "lo", 8, facing="east")
for k, net in enumerate(("lo4", "lo_4", "lo_5", "lo_6", "lo_7")):
    e = sp[f"end{k}"]
    t.wire(e, (e[0] + 20, e[1]))
    t.tunnel((e[0] + 20, e[1]), net, 4 if k == 0 else 1, facing="west")

# ---- generic parts with tunnels on their ports --------------------------------------------------


def hook(pt, net, width=1, side="L"):
    """A 20 px stub from a port to a tunnel (or Power / Ground)."""
    dx, dy = {"L": (-20, 0), "R": (20, 0), "T": (0, -20), "B": (0, 20)}[side]
    end = (pt[0] + dx, pt[1] + dy)
    t.wire(pt, end)
    if net in ("vcc", "gnd"):
        face = {"L": "west", "R": "east", "T": "north", "B": "south"}[side]
        t.add("#Wiring", "Power" if net == "vcc" else "Ground", end, facing=face, **({"width": width} if width > 1 else {}))
        return
    t.tunnel(end, net, width, facing={"L": "east", "R": "west", "T": "south", "B": "north"}[side])


def gate_not(loc, a, y):
    t.add("#Gates", "NOT Gate", loc)
    hook((loc[0] - 30, loc[1]), a)
    hook(loc, y, side="R")


def gate_and(loc, ins, y):
    """4-input AND (size 50); missing inputs tied high."""
    ins = list(ins) + ["vcc"] * (4 - len(ins))
    x, y0 = loc
    t.add("#Gates", "AND Gate", loc, inputs=4, size=50)
    for dy, j, row, name in zip((-20, -10, 10, 20), (0, 1, 1, 0), (-60, -30, 30, 60), ins):
        xv = x - 60 - 10 * j
        t.wire((x - 50, y0 + dy), (xv, y0 + dy), (xv, y0 + row), (x - 100, y0 + row))
        hook((x - 100, y0 + row), name)
    hook(loc, y, side="R")


def gate_or(loc, a, b, y):
    """y = a OR b = NOT(NOT a AND NOT b)."""
    x, y0 = loc
    gate_not((x - 250, y0 - 40), a, y + "_na")
    gate_not((x - 250, y0 + 40), b, y + "_nb")
    gate_and((x - 80, y0), [y + "_na", y + "_nb"], y + "_n")
    gate_not((x + 50, y0), y + "_n", y)


def buffer(loc, a, en, y, width=8):
    t.add("#Gates", "Controlled Buffer", loc, width=width)
    hook((loc[0] - 20, loc[1]), a, width)
    hook((loc[0] - 10, loc[1] + 10), en, side="B")
    hook(loc, y, width, side="R")


def register(loc, d, q, en, width):
    t.add("#Memory", "Register", loc, width=width, appearance="logisim_evolution")
    x, y = loc
    hook((x, y + 30), d, width)
    hook((x + 60, y + 30), q, width, side="R")
    hook((x, y + 70), "clk_5")
    hook((x, y + 50), en)
    hook((x + 30, y + 90), "gnd", side="B")


def dff(loc, d, q):
    t.add("#Memory", "D Flip-Flop", loc, appearance="logisim_evolution")
    x, y = loc
    hook((x - 10, y + 10), d)
    hook((x - 10, y + 50), "clk_5")
    hook((x + 50, y + 10), q, side="R")
    hook((x + 20, y + 60), "gnd", side="B")
    hook((x + 20, y), "gnd", side="T")


def rom(loc, a, q, table):
    words = " ".join(f"{v:x}" for v in table)
    t.add("#Memory", "ROM", loc, addrWidth=8, dataWidth=8, appearance="logisim_evolution",
          contents="addr/data: 8 8\n" + words + "\n")
    hook((loc[0], loc[1] + 10), a, 8)
    hook((loc[0] + 240, loc[1] + 60), q, 8, side="R")


# ---- IOD stub: port latches -----------------------------------------------------------------------
GX, GY = 400, 1900
gate_not((GX, GY), "rw", "rw_n")
gate_not((GX, GY + 60), "clk_wr_n", "wr")
gate_not((GX, GY + 120), "inr", "inr_n")
gate_and((GX + 300, GY), ["ie", "rw_n", "wr"], "port_we")
gate_and((GX + 300, GY + 200), ["ie", "rw"], "in_rd")
gate_or((GX + 700, GY + 400), "in_rd", "vec", "port_rd")
RX, RY = 1200, 1900
t.add("#Memory", "RAM", (RX, RY), addrWidth=8, dataWidth=8, enables="byte", trigger="high",
      databus="bibus", appearance="logisim_evolution", type="volatile")
hook((RX, RY + 10), "lo", 8)
hook((RX, RY + 80), "bus_data", 8)
hook((RX, RY + 50), "port_we")
t.wire((RX, RY + 60), (RX - 10, RY + 60))
t.add("#Wiring", "Power", (RX - 10, RY + 60), facing="west")
t.wire((RX + 240, RY + 80), (RX + 280, RY + 80))
t.tunnel((RX + 280, RY + 80), "port_q", 8, facing="west")
buffer((RX + 400, RY + 300), "port_q", "port_rd", "iod_bus_data")
t.text((RX + 100, RY - 40), "IOD stub: port latches (OUT writes, IN and the vector step read)")

# ---- IOD stub: the interrupt source -----------------------------------------------------------------
GX, GY = 400, 2800
gate_and((GX + 300, GY), ["lo_4", "lo_5", "lo_6", "lo_7"], "trig_port")
gate_and((GX + 300, GY + 200), ["ie", "rw_n", "trig_port"], "arm")
gate_not((GX + 300, GY + 400), "arm", "arm_n")
buffer((GX + 600, GY + 600), "bus_data", "arm", "cnt_d")
buffer((GX + 600, GY + 800), "dec_q", "arm_n", "cnt_d")
register((GX + 900, GY + 600), "cnt_d", "cnt", "vcc", 8)
register((GX + 900, GY + 900), "lo4", "pport", "arm", 4)
rom((GX + 1200, GY), "cnt", "dec_q", [max(k - 1, 0) for k in range(256)])
rom((GX + 1200, GY + 300), "cnt", "one_q", [int(k == 1) for k in range(256)])
X, Y = GX + 1600, GY + 360
sp = t.splitter((X, Y), 8, [[0]], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "one_q", 8, facing="east")
e = sp["end0"]
t.wire(e, (e[0] + 20, e[1]))
t.tunnel((e[0] + 20, e[1]), "fire", facing="west")
gate_and((GX + 300, GY + 1200), ["int", "inr_n"], "hold")
gate_or((GX + 700, GY + 1400), "fire", "hold", "int_d")
dff((GX + 900, GY + 1200), "int_d", "int")
gate_and((GX + 300, GY + 1600), ["int", "inr"], "vec_d")
dff((GX + 900, GY + 1600), "vec_d", "vec")
X, Y = GX + 1300, GY + 900
sp = t.splitter((X, Y), 8, [[0, 1, 2, 3], [4, 5, 6, 7]], facing="west", appear="left", spacing=2)
t.wire((X, Y), (X + 20, Y))
t.tunnel((X + 20, Y), "vec_addr", 8, facing="west")
e0, e1 = sp["end0"], sp["end1"]
t.wire(e0, (e0[0] - 20, e0[1]))
t.tunnel((e0[0] - 20, e0[1]), "pport", 4, facing="east")
t.wire(e1, (e1[0] - 20, e1[1]))
t.constant((e1[0] - 20, e1[1]), 0, 4)
buffer((GX + 1600, GY + 1000), "vec_addr", "vec", "iod_bus_addr_lo")
t.text((GX + 200, GY - 80), "IOD stub: OUT Fp, v (v > 0) raises INT v steps later for port p; vector step: 00:p, latch p")

# ---- output pins: main's, then the stub's -------------------------------------------------------------
OBS = cpu_design.OUTPUTS + [("int", 1), ("vec", 1)]
# halt (Logisim -tty stops when it is 1): tied low, cputest.py decides when the run is over
# (H set with I clear is not enough: an entry from the halt loop clears I one step before H)
t.output((4600, 40), "halt")
t.wire((4580, 40), (4600, 40))
t.add("#Wiring", "Ground", (4580, 40), facing="south")

y = 100
for name, w in OBS:
    t.output((4600, y), name, w)
    t.wire((4580, y), (4600, y))
    t.tunnel((4580, y), name, w, facing="east")
    y += 40

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    sys.exit(1 if bad else 0)
