#!/usr/bin/env python3
"""Combined bench -> circuits/mem/test_mem_cpu.circ (circuit test_mem_cpu)

The address section, the instruction register and the memory section wired
as in the CPU (notes/ir-mem-decisions.md M5, M6), driven by
`hbc2 vectors mem_cpu`: the real emulator runs a program, and the rows
drive the sections the way the control unit will (C16, C18-C22):

- the boot copy of the 64 KiB ROM (the program's power-on RAM image,
  written into the at28c256 model for the run) into the Logisim RAM;
- every instruction fetched from that RAM into the IR, PC := PC + 1 per byte;
- every RAM write of the emulator mirrored (address through bus A:B, the
  data from a stand-in for the register drivers, `cpu_data`);
- IN and OUT steps through the IOD stub, hardware interrupts: the stub
  drives the port and the data in the vector step, the IVT entry is read
  from RAM into V1, V2, and PC := V1:V2 through the IR's outputs.

Not here: the register section (bus A, bus B and its data-bus drivers are
bench pins), the flags, the control unit (the rows are its control words).
The IOD stub is two tri-state buffers (port onto bus_addr low, data onto
bus_data); its queue and port values live in the vector generator.

    python test_mem_cpu.py           # write, check
    python test_mem_cpu.py --test    # ... and run hbc2 vectors mem_cpu in Logisim
"""

import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project  # noqa: E402

NAME = "test_mem_cpu"
OUT = ROOT / "circuits" / "mem" / f"{NAME}.circ"

p = Project()
t = p.circuit(NAME)
libs = {m: p.library(ROOT / "circuits" / u / f"{m}.circ") for m, u in (("addr", "addr"), ("ir", "ir"), ("mem", "mem"))}


def place(m, loc):
    """A module instance; every port goes out 20 px to a tunnel (or is returned)."""
    return t.sub(libs[m], m, loc)


def side(u, port):
    xs = [q.loc[0] for q in u.ports]
    ys = [q.loc[1] for q in u.ports]
    x, y = u[port]
    if x == min(xs):
        return (-1, 0)
    if x == max(xs):
        return (1, 0)
    if y == min(ys):
        return (0, -1)
    return (0, 1)


def connect(u, port, net, width):
    d = side(u, port)
    x, y = u[port]
    end = (x + 20 * d[0], y + 20 * d[1])
    t.wire((x, y), end)
    facing = {(-1, 0): "east", (1, 0): "west", (0, -1): "south", (0, 1): "north"}[d]
    t.tunnel(end, net, width, facing=facing)


addr = place("addr", (1200, 600))
ir = place("ir", (1200, 2400))
mem = place("mem", (2600, 600))

# bench inputs: (name, width); the tunnel has the same name unless mapped
INPUTS = [("clk", 1), ("rst_n", 1), ("clk_mem_wr_n", 1),
          ("addr_base_sel", 2), ("addr_base_oe_n", 2), ("addr_off_sel", 1), ("addr_off_oe_n", 2), ("addr_inv", 1),
          ("addr_cin", 1), ("addr_lat_ld_n", 1), ("addr_pc_ld_n", 1), ("addr_sp_ld_n", 1), ("addr_lat_oe_n", 2),
          ("addr_pc_oe_n", 2), ("addr_sp_oe_n", 2), ("addr_bus_a", 8), ("addr_bus_b", 8),
          ("ir_b0_ld_n", 1), ("ir_b1_ld_n", 1), ("ir_v1_ld_n", 1), ("ir_v2_ld_n", 1),
          ("mem_re", 1), ("mem_oe_n", 1), ("mem_we_n", 1), ("mem_wbuf_oe_n", 1), ("mem_rom_oe_n", 1),
          ("iod_port", 8), ("iod_port_oe_n", 1), ("iod_data", 8), ("iod_data_oe_n", 1),
          ("cpu_data", 8), ("cpu_data_oe_n", 1)]
OUTPUTS = [("bus_data", 8), ("bus_addr_hi", 8), ("bus_addr_lo", 8), ("mem_d", 8), ("addr_cout", 1),
           ("ir_op", 6), ("ir_mode", 4), ("ir_v1", 8), ("ir_v2", 8)]

# module ports -> nets
for port, w in [(n, w) for n, w in INPUTS if n.startswith("addr_") or n in ("clk", "rst_n")]:
    if port in ("clk", "rst_n") or port in [q.name for q in addr.ports]:
        connect(addr, port, port, w)
connect(addr, "addr_v1", "ir_v1", 8)
connect(addr, "addr_v2", "ir_v2", 8)
connect(addr, "addr_bus_addr_lo_in", "bus_addr_lo", 8)
connect(addr, "addr_bus_addr_hi", "bus_addr_hi", 8)
connect(addr, "addr_bus_addr_lo", "bus_addr_lo", 8)
connect(addr, "addr_bus_out", "bus_data", 8)
connect(addr, "addr_cout", "addr_cout", 1)
for port in ("clk", "ir_b0_ld_n", "ir_b1_ld_n", "ir_v1_ld_n", "ir_v2_ld_n"):
    connect(ir, port, port, 1)
connect(ir, "ir_bus_in", "bus_data", 8)
for port, w in (("ir_op", 6), ("ir_mode", 4), ("ir_v1", 8), ("ir_v2", 8), ("ir_r1", 3), ("ir_r2", 3),
                ("ir_r3", 3), ("ir_r1_2_n", 1), ("ir_r2_2_n", 1), ("ir_r3_2_n", 1)):
    connect(ir, port, port, w)
for port in ("clk_mem_wr_n", "mem_re", "mem_oe_n", "mem_we_n", "mem_wbuf_oe_n", "mem_rom_oe_n"):
    connect(mem, port, port, 1)
connect(mem, "mem_addr", "bus_addr", 16)
connect(mem, "mem_bus_in", "bus_data", 8)
connect(mem, "mem_bus_out", "bus_data", 8)
connect(mem, "mem_d", "mem_d", 8)

# bus_addr (16) = bus_addr_hi : bus_addr_lo
X, Y = 400, 3400
sp = t.splitter((X, Y), 16, [list(range(8)), list(range(8, 16))], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "bus_addr", 16, facing="east")
for k, net in enumerate(("bus_addr_lo", "bus_addr_hi")):
    e = sp[f"end{k}"]
    t.wire(e, (e[0] + 20, e[1]))
    t.tunnel((e[0] + 20, e[1]), net, 8, facing="west")

# the IOD stub and the register stand-in: tri-state buffers onto the buses
for i, (src, en, net) in enumerate((("iod_port", "iod_port_oe_n", "bus_addr_lo"), ("iod_data", "iod_data_oe_n", "bus_data"),
                                    ("cpu_data", "cpu_data_oe_n", "bus_data"))):
    x, y = 600, 3600 + 120 * i
    t.tunnel((x - 40, y), src, 8, facing="east")
    t.wire((x - 40, y), (x - 20, y))
    t.add("#Gates", "Controlled Buffer", (x, y), width=8)
    t.add("#Gates", "NOT Gate", (x - 10, y + 40), facing="north")
    t.wire((x - 10, y + 40), (x - 10, y + 10))
    t.tunnel((x - 10, y + 70), en, facing="north")
    t.wire((x, y), (x + 20, y))
    t.tunnel((x + 20, y), net, 8, facing="west")

# bench pins
y = 100
for name, w in INPUTS:
    t.input((100, y), name, w)
    t.wire((100, y), (120, y))
    t.tunnel((120, y), name, w, facing="west")
    y += 40
y = 100
for name, w in OUTPUTS:
    t.output((3900, y), name, w)
    t.wire((3880, y), (3900, y))
    t.tunnel((3880, y), name, w, facing="east")
    y += 40

IN = dict(INPUTS)
OUTS = dict(OUTPUTS)

if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        # the ROM image is the program's: write it into the at28c256 model for the run
        exe = ROOT.parent / "hbc2" / "target" / "release" / "hbc2"
        subprocess.run(["cargo", "build", "--release", "-p", "hbc2-cli"], cwd=ROOT.parent / "hbc2", check=True)
        img = Path(tempfile.gettempdir()) / "hbc2_mem_cpu.rom"
        vec = Path(tempfile.gettempdir()) / "hbc2_mem_cpu.txt"
        env = {**__import__("os").environ, "HBC2_ROM_OUT": str(img)}
        subprocess.run([str(exe), "vectors", "mem_cpu", "-o", str(vec)], check=True, env=env)
        rom = [sys.executable, str(ROOT / "designs" / "at28c256.py")]
        subprocess.run(rom + ["--image", str(img)], check=True, stdout=subprocess.DEVNULL)
        try:
            bad += not logitest.opens(OUT)
            bad += not logitest.run_vectors(OUT, NAME, vec, timeout=3600)
        finally:
            subprocess.run(rom, check=True, stdout=subprocess.DEVNULL)    # back to the test pattern
    sys.exit(1 if bad else 0)
