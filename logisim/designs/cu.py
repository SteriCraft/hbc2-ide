#!/usr/bin/env python3
"""Control unit -> circuits/cu/cu.circ

The microcode sequencer of notes/cu-decisions.md (sections 1-6, 13, 14):

    address   A14-A9 opcode, A8-A5 mode, A4-A1 step, A0 cond
              opcode / mode: 3 x 74HCT157, the IR or bus_data while that byte
              is fetched (the bypass: SELECT = ir_b0_ld_n / ir_b1_ld_n, A =
              bus_data); /STROBE = cu_force gives row (00, 0)
              step: 74HCT175, the previous word's cu_next, cleared by rst_n
              cond: 74HCT153 half, by cu_cond_sel: 00 jc (74HCT151 by opcode
              bits 2..0: S Z F 0 C E N 1), 01 bnd_n = NOR3(HW, SW, H),
              10 hw_q, 11 x_n = NOR3(SW, cout_q, rst_early)
    microcode 7 x AT28C256 (the at28c256 model, IMAGE = 2..8: ucode0..6.bin)
    decode    before the pipeline (section 2): 74HCT138 (dbus), abus (2 AND,
              2 inverters, 1 NAND), the A / B latch enables, the constant
    clock     clk is 2 bits, two branches of clkgen's buffer: bit 0 the pipeline
              574s 1-5, bit 1 the 574s 6-8, both 175s and the 377 (section 14)
    pipeline  8 x 74HCT574, /OE = rst_rc (C24): off during reset, every line
              then held at its idle level by a pull resistor (the sequencer's
              cond_sel at 11, force at 0)
    state     74HCT175: hw_q (HW), cout_q (addr_cout), P (pending software
              interrupt: P_next = (P AND cu_swi_oe_n) OR NOT cu_swi_ld_n)
    pending   74HCT377 (from bus_data, /E = cu_swi_ld_n) + 74HCT541 onto the
              low byte of bus_addr (/OE = cu_swi_oe_n) (C13)

    python cu.py           # write, check, and write notes/bom-cu.csv
    python cu.py --test    # ... and run it in Logisim (hbc2 vectors cu)
    python cu.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "cu"
OUT = ROOT / "circuits" / "cu" / f"{M}.circ"
P = f"{M}_"

p = Project()
c = p.circuit(M)
rom_lib = p.library(ROOT / "circuits" / "chips" / "at28c256.circ")
reg_lib = p.library(ROOT / "circuits" / "chips" / "74574.circ")
chips = []


def n(name, k=None):
    return f"{P}{name}" if k is None else f"{P}{name}_{k}"


def rom(b):
    """Microcode bit b (EEPROM b // 8, I/O b % 8)."""
    return n("rom", b)


# ---- placement -----------------------------------------------------------------------------
TTL_X = {k: 600 + 420 * k for k in range(8)}
Y_ADDR, Y_ROM, Y_DEC, Y_PIPE, Y_STATE = 160, 760, 1160, 1760, 2160


def chip(part, col, y, pins, note=""):
    u = c.ttl(part, (TTL_X[col], y), facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    if note:
        c.text((TTL_X[col] - 10, y - 50), note)
    return u


def model(lib, part, x, y, pins, note, image=None):
    u = c.sub(lib, "ttl" + part, (x, y))
    chips.append(u)
    pin_tunnels(c, u, pins, levels=6, stagger=20)
    if image is not None:                       # simulation-only IMAGE pin: a constant (M3, Q10)
        px, py = u["IMAGE"]
        c.wire((px, py), (px - 20, py))
        c.constant((px - 20, py), image, 4, facing="east")
    c.text((x, y - 260), note)
    return u


# ---- address: 3 x 157, 151, 153, step 175 -----------------------------------------------------
def mux157(col, select, pairs, note):
    """pairs: (bus_data bit, IR net, address bit) for muxes 1..4; None = unused."""
    pins = {"SELECT": select, "nSTROBE": n("force")}
    for k, pr in enumerate(pairs, 1):
        if pr is None:
            pins.update({f"{k}A": "gnd", f"{k}B": "gnd"})
        else:
            bus, ir, a = pr
            pins.update({f"{k}A": n("bus_in", bus), f"{k}B": ir, f"{k}Y": n("a", a)})
    chip("74157", col, Y_ADDR, pins, note)


mux157(0, n("ir_b0_ld_n"), [(2 + k, n("ir_op", k), 9 + k) for k in range(4)], "opcode bits 3..0 (bypass: bus_data)")
mux157(1, n("ir_b0_ld_n"), [(6, n("ir_op", 4), 13), (7, n("ir_op", 5), 14), (0, n("ir_mode", 2), 7),
                           (1, n("ir_mode", 3), 8)], "opcode bits 5..4, mode bits 3..2")
mux157(2, n("ir_b1_ld_n"), [(6, n("ir_mode", 0), 5), (7, n("ir_mode", 1), 6), None, None],
       "mode bits 1..0 (with byte 1)")
chip("74151", 3, Y_ADDR, {"A": n("ir_op", 0), "B": n("ir_op", 1), "C": n("ir_op", 2), "nG": "gnd",
                         "D0": n("flag_s"), "D1": n("flag_z"), "D2": n("flag_f"), "D3": "gnd",
                         "D4": n("flag_c"), "D5": n("flag_e"), "D6": n("flag_n"), "D7": "vcc", "Y": n("jc")},
     "jump condition: S Z F 0 C E N 1 by opcode bits 2..0")
chip("74153", 4, Y_ADDR, {"S0": n("cond_sel", 0), "S1": n("cond_sel", 1), "n1E": "gnd",
                         "1D0": n("jc"), "1D1": n("bnd_n"), "1D2": n("hw_q"), "1D3": n("x_n"), "1Y": n("a", 0),
                         "n2E": "vcc", "2D0": "gnd", "2D1": "gnd", "2D2": "gnd", "2D3": "gnd"},
     "cond: jc, bnd_n, hw_q, x_n")
step = {"nCLR": "rst_n", "CLK": "clk_1"}
for k in range(4):
    step.update({f"D{k + 1}": rom(48 + k), f"Q{k + 1}": n("a", 1 + k)})
chip("74175", 5, Y_ADDR, step, "step register (cu_next)")

# ---- microcode: 7 x AT28C256 -----------------------------------------------------------------
for k in range(7):
    pins = {f"A{j}": n("a", j) for j in range(15)}
    pins.update({"/CE": "gnd", "/OE": "gnd", "/WE": "vcc"})
    for j in range(8):
        if 8 * k + j < 54:                       # bits 54, 55 spare
            pins[f"I/O{j}"] = rom(8 * k + j)
    model(rom_lib, "at28c256", 600 + 520 * k, Y_ROM, pins, f"microcode bits {8 * k + 7}..{8 * k} (IMAGE {2 + k})",
          image=2 + k)

# ---- decode before the pipeline (section 2) --------------------------------------------------
chip("74138", 0, Y_DEC, {"A": rom(19), "B": rom(20), "C": rom(21), "G1": "vcc", "nG2A": "gnd", "nG2B": "gnd",
                        **{f"nY{k}": n("dbus_n", k) for k in range(1, 8)}},
     "dbus: the bus_data driver")
chip("7408", 1, Y_DEC, {"1A": rom(23), "1B": n("ab0_n"), "1Y": n("lat_hi_off"),
                       "2A": n("ab1_n"), "2B": rom(22), "2Y": n("ie_d"),
                       "3A": n("flag_i"), "3B": n("int"), "3Y": n("hw"),
                       "4A": n("flag_i"), "4B": n("p"), "4Y": n("sw")},
     "abus: latch high off, IE; HW, SW")
# P_next = (P AND cu_swi_oe_n) OR NOT cu_swi_ld_n, after the pipeline (slack: section 14)
chip("7408", 2, Y_DEC, {"1A": rom(16), "1B": rom(17), "1Y": n("bsel3"),
                       "2A": n("p"), "2B": n("swi_oe_n"), "2Y": n("p_kept"),
                       "3A": "gnd", "3B": "gnd", "4A": "gnd", "4B": "gnd"},
     "b_sel = 11; P kept")
chip("7414", 3, Y_DEC, {"1A": rom(3), "1Y": n("const_0_d"), "2A": rom(22), "2Y": n("ab0_n"),
                       "3A": rom(23), "3Y": n("ab1_n"), "4A": n("swi_ld_n"), "4Y": n("swi_ld"),
                       "5A": n("p_nor"), "5Y": n("p_next"), "6A": "gnd"},
     "NOT alu_op3, abus bits; P")
chip("7400", 4, Y_DEC, {"1A": rom(22), "1B": rom(23), "1Y": n("swi_oe_n_d"),
                       "2A": rom(14), "2B": rom(15), "2Y": n("alat_oe_n_d"),
                       "3A": n("bsel3"), "3B": rom(18), "3Y": n("blat_oe_n_d"),
                       "4A": "gnd", "4B": "gnd"},
     "abus 11, A latch, B latch")
chip("7427", 5, Y_DEC, {"1A": n("hw"), "1B": n("sw"), "1C": n("flag_h"), "1Y": n("bnd_n"),
                       "2A": n("sw"), "2B": n("cout_q"), "2C": "rst_early", "2Y": n("x_n"),
                       "3A": n("p_kept"), "3B": n("swi_ld"), "3C": "gnd", "3Y": n("p_nor")},
     "bnd_n, x_n; P")

# ---- pipeline: 8 x 74HCT574 ------------------------------------------------------------------
# (Q net, D source, pulled level): the outputs are named as the receiving section
# names them, with the cu_ prefix (vectors/cu.rs CU_OUT); pulled = the idle word
# decoded, cond_sel 11, force 0 (C24, word::pulled).
def out(name, k=None):
    return n(name, k)


PIPE = (
    [(out("alu_op", k), rom(k), 0) for k in range(4)]
    + [(out("flags_op", k), rom(4 + k), 0) for k in range(4)]
    + [(out("flags_bit_val"), rom(8), 0), (out("flags_oe_n"), n("dbus_n", 3), 1), (out("regs_we_n"), rom(9), 1)]
    + [(out("regs_a_sel", k), rom(10 + k), 1) for k in range(2)]
    + [(out("regs_cu_idx", k), rom(12 + k), 0) for k in range(2)]
    + [(out("regs_cu_bank_n", k), rom(14 + k), 1) for k in range(2)]
    + [(out("regs_b_sel", k), rom(16 + k), 1) for k in range(2)]
    + [(out("regs_a_lat_oe_n"), n("alat_oe_n_d"), 0), (out("regs_b_lat_oe_n"), n("blat_oe_n_d"), 0),
       (out("regs_const_oe_n"), rom(18), 1), (out("regs_const_0"), n("const_0_d"), 1),
       (out("regs_a_bus_oe_n"), n("dbus_n", 1), 1), (out("regs_res_oe_n"), n("dbus_n", 2), 1)]
    + [(out("addr_base_sel", k), rom(24 + k), 0) for k in range(2)]
    + [(out("addr_base_oe_n", k), rom(26 + k), 1) for k in range(2)]
    + [(out("addr_off_sel"), rom(28), 0)]
    + [(out("addr_off_oe_n", k), rom(29 + k), 1) for k in range(2)]
    + [(out("addr_inv"), rom(31), 0), (out("addr_cin"), rom(32), 0), (out("addr_lat_ld_n"), rom(33), 1),
       (out("addr_pc_ld_n"), rom(34), 1), (out("addr_sp_ld_n"), rom(35), 1),
       (out("addr_lat_oe_n", 0), rom(23), 0), (out("addr_lat_oe_n", 1), n("lat_hi_off"), 0),
       (out("addr_pc_oe_n", 0), n("dbus_n", 5), 1), (out("addr_pc_oe_n", 1), n("dbus_n", 4), 1),
       (out("addr_sp_oe_n", 0), n("dbus_n", 7), 1), (out("addr_sp_oe_n", 1), n("dbus_n", 6), 1)]
    + [(n(f"ir_{b}_ld_n"), rom(36 + k), 1) for k, b in enumerate(("b0", "b1", "v1", "v2"))]
    + [(out("mem_re"), rom(40), 0), (out("mem_oe_n"), rom(41), 1), (out("mem_we_n"), rom(42), 1),
       (out("mem_wbuf_oe_n"), rom(43), 1), (out("mem_rom_oe_n"), rom(44), 1),
       (out("ie"), n("ie_d"), 0), (out("rw"), n("dbus_n", 1), 1), (out("inr"), rom(45), 0)]
    + [(n("swi_ld_n"), rom(46), 1), (n("swi_oe_n"), n("swi_oe_n_d"), 1), (n("force"), rom(47), 0),
       (n("cond_sel", 0), rom(52), 1), (n("cond_sel", 1), rom(53), 1)]
)
assert len(PIPE) == 60 and len({q for q, _, _ in PIPE}) == 60
for i in range(8):
    pins = {"/OE": "rst_rc", "CP": "clk_0" if i < 5 else "clk_1"}    # two clock branches (section 14)
    for k in range(8):
        j = 8 * i + k
        if j < len(PIPE):
            q, d, _ = PIPE[j]
            pins.update({f"D{k}": d, f"Q{k}": q})
        else:
            pins[f"D{k}"] = "gnd"                 # 4 spare bits
    first, last = PIPE[8 * i][0], PIPE[min(8 * i + 7, 59)][0]
    model(reg_lib, "74574", 540 + 520 * i, Y_PIPE, pins, f"pipeline {i + 1}: {first[3:]} .. {last[3:]}")

# pull resistors (one per line: the networks of C24), in a column on the right
PX, PY = 600 + 520 * 8 + 200, Y_ROM - 200
for j, (q, _, level) in enumerate(PIPE):
    x, y = PX + 260 * (j // 20), PY + 40 * (j % 20)
    c.add("#Wiring", "Pull Resistor", (x, y), facing="east", pull=str(level))
    c.wire((x, y), (x + 30, y))
    c.tunnel((x + 30, y), q, facing="west")
c.text((PX, PY - 40), "pull resistors (C24): the idle level of each pipeline line while rst_rc holds the 574s off")

# ---- state: 175, pending port latch: 377 + 541 -------------------------------------------------
chip("74175", 0, Y_STATE, {"nCLR": "rst_n", "CLK": "clk_1", "D1": n("hw"), "Q1": n("hw_q"),
                          "D2": n("addr_cout"), "Q2": n("cout_q"), "D3": n("p_next"), "Q3": n("p"), "D4": "gnd"},
     "hw_q, cout_q, P")
latch = {"nCLKen": n("swi_ld_n"), "CLK": "clk_1"}
buf = {"nOE1": n("swi_oe_n"), "nOE2": "gnd"}
for k in range(8):
    latch.update({f"D{k + 1}": n("bus_in", k), f"Q{k + 1}": n("swi", k)})
    buf.update({f"A{k + 1}": n("swi", k), f"Y{k + 1}": n("bus_addr_lo", k)})
chip("74377", 1, Y_STATE, latch, "pending software interrupt port (C13)")
chip("74541", 2, Y_STATE, buf, "pending port -> bus_addr low byte")

# chips numbered top to bottom, then left to right (by body: models are anchored at pin 1, bottom left)
def body(u):
    x, y = u.loc
    return (y - 100, x) if u.name.startswith("ttl") else (y, x)


for i, u in enumerate(sorted(chips, key=body), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ---------------------------------------------------------------------------
TOP = [("clk", 2), ("rst_n", 1), ("rst_early", 1), ("rst_rc", 1)]     # clk: clkgen branches 2 and 3
LEFT = ([(n("bus_in"), 8), (n("ir_op"), 6), (n("ir_mode"), 4)]
        + [(n("flag", f), 1) for f in "ceinszfh"] + [(n("int"), 1), (n("addr_cout"), 1)])
WIDE = {"alu_op": 4, "flags_op": 4, "regs_a_sel": 2, "regs_cu_idx": 2, "regs_cu_bank_n": 2, "regs_b_sel": 2,
        "addr_base_sel": 2, "addr_base_oe_n": 2, "addr_off_oe_n": 2, "addr_lat_oe_n": 2, "addr_pc_oe_n": 2,
        "addr_sp_oe_n": 2}
INSIDE = {n("swi_ld_n"), n("swi_oe_n"), n("force"), n("cond_sel", 0), n("cond_sel", 1)}   # stay inside cu
RIGHT = []
for q, _, _ in PIPE:
    base, _, bit = q.rpartition("_")
    if q in INSIDE:
        continue
    if base[len(P):] in WIDE:
        if bit == "0":
            RIGHT.append((base, WIDE[base[len(P):]]))
    else:
        RIGHT.append((q, 1))
RIGHT.append((n("bus_addr_lo"), 8))
X_IN, y = 220, 120
for name, w in TOP + LEFT:
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = PX + 900, 120
for name, w in RIGHT:
    bus_tunnels(c, (X_OUT, y), name, w, output=True)
    y += 60 if w == 1 else 20 * w + 60

c.symbol(top=[nm for nm, _ in TOP], left=[nm for nm, _ in LEFT], right=[nm for nm, _ in RIGHT])

INPUTS = dict(TOP + LEFT)
OUTPUTS = dict(RIGHT)

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-cu.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-cu.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "cu")
    sys.exit(1 if bad else 0)
