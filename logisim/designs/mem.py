#!/usr/bin/env python3
"""Memory and bus interface -> circuits/mem/mem.circ

64 KiB of RAM and the 64 KiB boot ROM on a local data bus (mem_d), joined
to the data bus by two 74HCT541. Why it is built this way:
notes/ir-mem-decisions.md sections 3-4 and M1-M4.

    RAM   AS6C1008 (128 K x 8, A16 = 0): CE# = 0, CE2 = mem_re, OE# = mem_oe_n,
          WE# = clk_mem_wr_n OR mem_we_n (74HCT32): low about 125-375 ns (M1)
    ROM   2 x AT28C256 on their own net mem_rom_d: /CE = A15 (low chip) or
          NOT A15 (74HCT14, high chip), /OE = mem_rom_oe_n (off outside the
          boot copy, M8), /WE = 1; a 74HCT541 drives mem_rom_d onto mem_d
          when mem_rom_oe_n = 0 (levels, M4)
    read  74HCT541 mem_d -> bus_data (mem_bus_out), /OE = mem_oe_n
    write 74HCT541 bus_data (mem_bus_in) -> mem_d, /OE = mem_wbuf_oe_n

The control unit drives the strobes in the combinations of section 4 (C19);
RW and IE go to the IOD connector only.

    python mem.py           # write, check, and write notes/bom-mem.csv
    python mem.py --test    # ... and run it in Logisim (hbc2 vectors mem)
    python mem.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import circcheck  # noqa: E402
import netlist as nl  # noqa: E402
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "mem"
OUT = ROOT / "circuits" / "mem" / f"{M}.circ"
P = f"{M}_"

p = Project()
c = p.circuit(M)
ram_lib = p.library(ROOT / "circuits" / "chips" / "as6c1008.circ")
rom_lib = p.library(ROOT / "circuits" / "chips" / "at28c256.circ")
chips = []


def n(name, k=None):
    return f"{P}{name}" if k is None else f"{P}{name}_{k}"


def extras(u, names):
    """Simulation-only pins on a model's left edge, 10 px apart: staggered stubs."""
    for k, (pin, net) in enumerate(names.items()):
        x, y = u[pin]
        end = (x - (20 if k % 2 == 0 else 100), y)
        c.wire((x, y), end)
        if isinstance(net, tuple):                   # ("const", value, width)
            c.constant(end, net[1], net[2], facing="east")
        elif net in ("gnd", "vcc"):
            c.add("#Wiring", "Ground" if net == "gnd" else "Power", end, facing="west")
        else:
            c.tunnel(end, net, facing="east")


def model(lib, part, loc, pins, extra, note):
    u = c.sub(lib, "ttl" + part, loc)
    chips.append(u)
    pin_tunnels(c, u, pins, levels=6, stagger=20)
    extras(u, extra)
    c.text((loc[0], loc[1] - 260), note)
    return u


def ttl(part, loc, pins, note):
    u = c.ttl(part, loc, facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    c.text((loc[0] - 10, loc[1] - 50), note)
    return u


# ---- RAM --------------------------------------------------------------------------------------
pins = {f"A{k}": n("addr", k) for k in range(16)}
pins.update({"A16": "gnd", "CE#": "gnd", "CE2": n("re"), "OE#": n("oe_n"), "WE#": n("ram_we_n")})
pins.update({f"DQ{k}": n("d", k) for k in range(8)})
model(ram_lib, "as6c1008", (800, 500), pins, {f"DQ{k}_in": n("d", k) for k in range(8)},
      "RAM 0000-FFFF (A16 = 0); DQ_in: the same pins, read side (M2)")

# ---- boot ROM: two halves on mem_rom_d ----------------------------------------------------------
for i, (ce, what) in enumerate(((n("addr", 15), "0000-7FFF"), (n("a15_n"), "8000-FFFF"))):
    pins = {f"A{k}": n("addr", k) for k in range(15)}
    pins.update({"/CE": ce, "/OE": n("rom_oe_n"), "/WE": "vcc"})   # M8
    pins.update({f"I/O{k}": n("rom_d", k) for k in range(8)})
    model(rom_lib, "at28c256", (800 + 520 * i, 1000), pins, {"IMAGE": ("const", i, 4)},
          f"boot ROM {what} (IMAGE = {i}: its half of the boot image, M3)")

# ---- glue: 3 x 74HCT541, 74HCT32, 74HCT14 -------------------------------------------------------
X, Y = 800, 1400


def buf541(col, a, y, oe, note):
    pins = {"nOE1": oe, "nOE2": "gnd"}
    for k in range(8):
        pins.update({f"A{k + 1}": f"{a}_{k}", f"Y{k + 1}": f"{y}_{k}"})
    ttl("74541", (X + 420 * col, Y), pins, note)


buf541(0, n("rom_d"), n("d"), n("rom_oe_n"), "ROM → local bus (boot copy; HCT levels, M4)")
buf541(1, n("bus_in"), n("d"), n("wbuf_oe_n"), "write: data bus → local bus")
buf541(2, n("d"), n("bus_out"), n("oe_n"), "read: local bus → data bus")
ttl("7432", (X + 420 * 3, Y), {"1A": "clk_mem_wr_n", "1B": n("we_n"), "1Y": n("ram_we_n"),
                               "2A": "gnd", "2B": "gnd", "3A": "gnd", "3B": "gnd", "4A": "gnd", "4B": "gnd"},
    "RAM WE# = clk_mem_wr_n OR mem_we_n (M1)")
ttl("7414", (X + 420 * 4, Y), {"1A": n("addr", 15), "1Y": n("a15_n"), **{f"{g}A": "gnd" for g in range(2, 7)}},
    "NOT A15: the high ROM's /CE")


def body_top_left(u):
    if u.lib is not None and not isinstance(u.lib, str):          # a chip model: DIP anchor = pin 1
        return u.loc[1] - 100, u.loc[0]
    x1, y1, _, _ = circcheck.ttl_body(nl.Comp("ttl", u.name, "#TTL", u.loc, {"facing": "south"}))
    return y1, x1


for i, u in enumerate(sorted(chips, key=body_top_left), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ---------------------------------------------------------------------------
TOP = [("clk_mem_wr_n", 1), (P + "re", 1), (P + "oe_n", 1), (P + "we_n", 1), (P + "wbuf_oe_n", 1),
       (P + "rom_oe_n", 1)]
LEFT = [(P + "addr", 16), (P + "bus_in", 8)]
RIGHT = [(P + "bus_out", 8), (P + "d", 8)]
X_IN, y = 220, 120
for name, w in TOP + LEFT:
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = X + 420 * 5 + 200, 200
for name, w in RIGHT:
    bus_tunnels(c, (X_OUT, y), name, w, output=True)
    y += 20 * w + 60

c.symbol(top=[n for n, _ in TOP], left=[n for n, _ in LEFT], right=[n for n, _ in RIGHT])

INPUTS = dict(TOP + LEFT)
OUTPUTS = dict(RIGHT)

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-mem.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-mem.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import render
        print(render.render(nl.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "mem")
    sys.exit(1 if bad else 0)
