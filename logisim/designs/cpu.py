#!/usr/bin/env python3
"""The whole CPU -> logisim/cpu.circ (circuit main)

`main` only connects the sections (conventions.md): clkgen, cu, alu, flags,
regs, addr, ir, mem, each port to a tunnel named after its net.

    nets      the control unit's outputs cu_<x> drive the nets <x> (U12);
              bus_data joins every *_bus_in and *_bus_out; bus_addr is
              bus_addr_hi : bus_addr_lo; bus A / bus B are regs_a / regs_b
    clocks    clkgen's 6 branches (cu-decisions.md 14.1): clk_0 ir, clk_1
              addr, clk_2 + clk_3 cu, clk_4 regs + flags, clk_5 the reset
              flip-flops (inside clkgen) and the IOD (the connector)
    pins      inputs: osc (the oscillator), rc (the reset RC node), and the
              IOD connector: int, and what the IOD drives onto bus_data and the
              low byte of bus_addr (iod_bus_data, iod_bus_addr_lo: a Logisim
              subcircuit pin is one-way, so the bidirectional connector line is
              an input joined to the bus net plus the bus as an output, as the
              RAM model's DQ pins, ir-mem-decisions.md M2)
              outputs: the connector (bus_data, bus_addr, rw, ie, inr, clk,
              clk_wr_n, rst_n), the internal buses (bus_a, bus_b, mem_d: the
              contention check, conventions.md), and test points for the trace
              bench: every control line, the IR fields, the flags (U14)

    python cpu.py           # write, check, and write notes/bom-cpu.csv
    python cpu.py --test    # ... and the trace tests: the programs in tests/cpu and 10 random
                            # ones against the emulator (tools/cputest.py suite, about 20 minutes)
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project  # noqa: E402

OUT = ROOT / "cpu.circ"
p = Project()
t = p.circuit("main")
UNIT = {"alu": "alu", "flags": "flags", "regs": "registers", "addr": "addr", "ir": "ir", "mem": "mem",
        "clkgen": "cu", "cu": "cu"}
libs = {m: p.library(ROOT / "circuits" / u / f"{m}.circ") for m, u in UNIT.items()}

# the control unit's outputs (cu_<x> -> <x>), in cu's order
CU_OUT = [("alu_op", 4), ("flags_op", 4), ("flags_bit_val", 1), ("flags_oe_n", 1), ("regs_we_n", 1),
          ("regs_a_sel", 2), ("regs_cu_idx", 2), ("regs_cu_bank_n", 2), ("regs_b_sel", 2), ("regs_a_lat_oe_n", 1),
          ("regs_b_lat_oe_n", 1), ("regs_const_oe_n", 1), ("regs_const_0", 1), ("regs_a_bus_oe_n", 1),
          ("regs_res_oe_n", 1), ("addr_base_sel", 2), ("addr_base_oe_n", 2), ("addr_off_sel", 1),
          ("addr_off_oe_n", 2), ("addr_inv", 1), ("addr_cin", 1), ("addr_lat_ld_n", 1), ("addr_pc_ld_n", 1),
          ("addr_sp_ld_n", 1), ("addr_lat_oe_n", 2), ("addr_pc_oe_n", 2), ("addr_sp_oe_n", 2), ("ir_b0_ld_n", 1),
          ("ir_b1_ld_n", 1), ("ir_v1_ld_n", 1), ("ir_v2_ld_n", 1), ("mem_re", 1), ("mem_oe_n", 1), ("mem_we_n", 1),
          ("mem_wbuf_oe_n", 1), ("mem_rom_oe_n", 1), ("ie", 1), ("rw", 1), ("inr", 1)]
FLAGS = ["flag_c", "flag_e", "flag_i", "flag_n", "flag_s", "flag_z", "flag_f", "flag_h"]   # bit 0 .. 7
IR_OUT = [("ir_op", 6), ("ir_mode", 4), ("ir_r1", 3), ("ir_r2", 3), ("ir_r3", 3), ("ir_r1_2_n", 1),
          ("ir_r2_2_n", 1), ("ir_r3_2_n", 1), ("ir_v1", 8), ("ir_v2", 8)]

# module port -> net (width from the port); a port missing here is an error
NETS = {
    "clkgen": {"clkgen_osc": "osc", "clkgen_rc": "rc", "clk": "clk", "clk_wr_n": "clk_wr_n",
               "clk_mem_wr_n": "clk_mem_wr_n", "rst_n": "rst_n", "rst_early": "rst_early", "rst_rc": "rst_rc"},
    "cu": {"clk": ("clk_2", "clk_3"), "rst_n": "rst_n", "rst_early": "rst_early", "rst_rc": "rst_rc",
           "cu_bus_in": "bus_data", "cu_ir_op": "ir_op", "cu_ir_mode": "ir_mode",
           **{f"cu_{f}": f for f in FLAGS}, "cu_int": "int", "cu_addr_cout": "addr_cout",
           **{f"cu_{x}": x for x, _ in CU_OUT}, "cu_bus_addr_lo": "bus_addr_lo"},
    "alu": {"alu_op": "alu_op", "alu_cin": "flag_c", "alu_a": "bus_a", "alu_b": "bus_b",
            **{f"alu_{f}": f"alu_{f}" for f in "nzsefc"}, "alu_r": "alu_r"},
    "flags": {"flags_op": "flags_op", "flags_bit_val": "flags_bit_val", "flags_oe_n": "flags_oe_n",
              "clk": "clk_4", "rst_n": "rst_n", **{f"flags_alu_{f}": f"alu_{f}" for f in "cznsef"},
              "flags_bus_in": "bus_data", **{f: f for f in FLAGS}, "flags_bus_out": "bus_data"},
    "regs": {"clk": "clk_4", "clk_wr_n": "clk_wr_n",
             **{x: x for x, _ in CU_OUT if x.startswith("regs_")},
             **{f"regs_{f}": f"ir_{f}" for f in ("r1", "r2", "r3", "r1_2_n", "r2_2_n", "r3_2_n")},
             "regs_alu_r": "alu_r", "regs_bus_in": "bus_data", "regs_a": "bus_a", "regs_b": "bus_b",
             "regs_bus_out": "bus_data"},
    "addr": {"clk": "clk_1", "rst_n": "rst_n", **{x: x for x, _ in CU_OUT if x.startswith("addr_")},
             "addr_bus_a": "bus_a", "addr_bus_b": "bus_b", "addr_v1": "ir_v1", "addr_v2": "ir_v2",
             "addr_bus_addr_lo_in": "bus_addr_lo", "addr_bus_addr_hi": "bus_addr_hi",
             "addr_bus_addr_lo": "bus_addr_lo", "addr_bus_out": "bus_data", "addr_cout": "addr_cout"},
    "ir": {"clk": "clk_0", **{x: x for x, _ in CU_OUT if x.startswith("ir_")}, "ir_bus_in": "bus_data",
           **{x: x for x, _ in IR_OUT}},
    "mem": {"clk_mem_wr_n": "clk_mem_wr_n", **{x: x for x, _ in CU_OUT if x.startswith("mem_")},
            "mem_addr": "bus_addr", "mem_bus_in": "bus_data", "mem_bus_out": "bus_data", "mem_d": "mem_d"},
}

INPUTS = [("osc", 1), ("rc", 1), ("int", 1), ("iod_bus_data", 8), ("iod_bus_addr_lo", 8)]
OUTPUTS = ([("clk", 6), ("clk_wr_n", 1), ("clk_mem_wr_n", 1), ("rst_n", 1), ("rst_early", 1), ("rst_rc", 1),
            ("bus_data", 8), ("bus_addr", 16), ("bus_a", 8), ("bus_b", 8), ("mem_d", 8), ("alu_r", 8),
            ("flag_reg", 8), ("addr_cout", 1)]
           + [(x, w) for x, w in CU_OUT] + IR_OUT)


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


FACING = {(-1, 0): "east", (1, 0): "west", (0, -1): "south", (0, 1): "north"}


def connect(u, port, net, width):
    d = side(u, port)
    x, y = u[port]
    if isinstance(net, tuple):           # a multi-bit port fed from 1-bit nets: a splitter at the port
        assert d == (0, -1), port        # (cu's clk, on top)
        y0 = y - 20
        t.wire((x, y), (x, y0))
        sp = t.splitter((x, y0), width, [[k] for k in range(width)], facing="north", appear="left", spacing=2)
        for k, n in enumerate(net):
            e = sp[f"end{k}"]
            t.wire(e, (e[0], e[1] - 20))
            t.tunnel((e[0], e[1] - 20), n, facing="south")
        return
    end = (x + 20 * d[0], y + 20 * d[1])
    t.wire((x, y), end)
    t.tunnel(end, net, width, facing=FACING[d])


x = 900
inst = {}
for m in ("clkgen", "cu", "alu", "flags", "regs", "addr", "ir", "mem"):
    probe = t.sub(libs[m], m, (0, 0))
    t.parts.remove(probe)
    w = -min(q.loc[0] for q in probe.ports)
    x += w + 200
    inst[m] = u = t.sub(libs[m], m, (x, 400))
    ports = {q.name: q.width for q in u.ports}
    missing = set(ports) - set(NETS[m])
    extra = set(NETS[m]) - set(ports)
    assert not missing and not extra, (m, missing, extra)
    for port, net in NETS[m].items():
        connect(u, port, net, ports[port])
    x += 200

# clk (6) -> clk_0 .. clk_5; flags; bus_addr = hi : lo
X, Y = 300, 1800
sp = t.splitter((X, Y), 6, [[k] for k in range(6)], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "clk", 6, facing="east")
for k in range(6):
    e = sp[f"end{k}"]
    t.wire(e, (e[0] + 20, e[1]))
    t.tunnel((e[0] + 20, e[1]), f"clk_{k}", facing="west")
X, Y = 300, 2200
sp = t.splitter((X, Y), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "flag_reg", 8, facing="east")
for k, f in enumerate(FLAGS):
    e = sp[f"end{k}"]
    t.wire(e, (e[0] + 20, e[1]))
    t.tunnel((e[0] + 20, e[1]), f, facing="west")
X, Y = 300, 2700
sp = t.splitter((X, Y), 16, [list(range(8)), list(range(8, 16))], facing="east", appear="right", spacing=2)
t.tunnel((X, Y), "bus_addr", 16, facing="east")
for k, net in enumerate(("bus_addr_lo", "bus_addr_hi")):
    e = sp[f"end{k}"]
    t.wire(e, (e[0] + 20, e[1]))
    t.tunnel((e[0] + 20, e[1]), net, 8, facing="west")

# pins: the IOD's drives join the bus nets
y = 100
for name, w in INPUTS:
    net = {"iod_bus_data": "bus_data", "iod_bus_addr_lo": "bus_addr_lo"}.get(name, name)
    t.input((100, y), name, w)
    t.wire((100, y), (120, y))
    t.tunnel((120, y), net, w, facing="west")
    y += 40
y = 3200
for name, w in OUTPUTS:
    t.output((300, y), name, w)
    t.wire((280, y), (300, y))
    t.tunnel((280, y), name, w, facing="east")
    y += 40
t.text((100, 3160), "test points for the trace bench (circuits/cpu/test_cpu.circ), U14")

if __name__ == "__main__":
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), "main")
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-cpu.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-cpu.csv")
    bad += len(errors)
    if "--test" in sys.argv:
        import subprocess
        cmd = [sys.executable, str(ROOT / "tools" / "cputest.py"), "suite", "--random", "10", "--jobs", "8"]
        bad += subprocess.run(cmd).returncode != 0
    sys.exit(1 if bad else 0)
