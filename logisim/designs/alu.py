#!/usr/bin/env python3
"""ALU -> circuits/alu/alu.circ

Groups the ALU blocks (each a library file in circuits/alu/):

    add_sub8 ADD ADC SUB SBB (INC / DEC = ADD / SUB with b = 01)
    logic8   AND OR XOR
    shift8   SHL / SHR / ASR wiring, carry select, ASR fill
    rmux8    result multiplexer (tri-state buffers, NOT a)
    flags8   Z N of the result
    cmp8     S E F (CMP), from a and b

alu_op (op3 op2 op1 op0) drives them directly, without any glue logic:

    op3 = 0: arithmetic, op0 = sub, op1 = use_c, op2 no effect (01xx = 00xx)
        0000 ADD (INC with b = 01)   0001 SUB, CMP (DEC with b = 01)
        0010 ADC   0011 SBB
    op3 = 1: 1000 AND  1001 OR  1010 XOR  1011 NOT  1100 SHL  1110 SHR  1111 ASR
        (1101 = ASR)

INC and DEC need the control unit to put 01 on alu_b (alu-decisions.md D16).

Outputs: alu_r and every flag the ALU can produce. The control unit decides
which flags to store: C Z N for arithmetic and SHL/ASR, Z N for logic and SHR,
S E F for CMP. cmp8 compares a and b whatever alu_op is.

The blocks talk through tunnels named alu_<signal>; inputs on the left,
alu_r on the right, flags at the bottom.

    python alu.py           # write, check, and write notes/bom-alu.csv
    python alu.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python alu.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project  # noqa: E402

M = "alu"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)
lib = {m: p.library(OUT.parent / f"{m}.circ") for m in ("add_sub8", "logic8", "shift8", "rmux8", "flags8", "cmp8")}

STEP = {"left": (-20, 0), "right": (20, 0), "top": (0, -20), "bottom": (0, 20)}
FACING = {"left": "east", "right": "west", "top": "south", "bottom": "north"}


def link(u, module, port, side, signal, width=1):
    """Port `port` of instance u -> 20 px stub -> tunnel alu_<signal>."""
    at = u[f"{module}_{port}"]
    end = (at[0] + STEP[side][0], at[1] + STEP[side][1])
    c.wire(at, end)
    c.tunnel(end, f"{M}_{signal}", width, facing=FACING[side])


# ---- blocks ----------------------------------------------------------------------
u = c.sub(lib["add_sub8"], "add_sub8", (1000, 300))
for port, sig in (("sub", "op_0"), ("use_c", "op_1"), ("cin", "cin")):
    link(u, "add_sub8", port, "top", sig)
link(u, "add_sub8", "a", "left", "a", 8)
link(u, "add_sub8", "b", "left", "b", 8)
link(u, "add_sub8", "s", "right", "sum", 8)
link(u, "add_sub8", "c", "bottom", "c_add")

u = c.sub(lib["logic8"], "logic8", (1000, 620))
link(u, "logic8", "a", "left", "a", 8)
link(u, "logic8", "b", "left", "b", 8)
for port in ("and", "or", "xor"):
    link(u, "logic8", port, "right", port, 8)

u = c.sub(lib["shift8"], "shift8", (1000, 900))
link(u, "shift8", "sel", "top", "op_3")
link(u, "shift8", "asr", "top", "op_0")
link(u, "shift8", "c_add", "top", "c_add")
link(u, "shift8", "a", "left", "a", 8)
link(u, "shift8", "shl", "right", "shl", 8)
link(u, "shift8", "sr", "right", "sr", 8)
c_out = u["shift8_c"]

u = c.sub(lib["rmux8"], "rmux8", (1600, 520))
link(u, "rmux8", "op", "top", "op", 4)
for port in ("sum", "and", "or", "xor", "a", "shl", "sr"):
    link(u, "rmux8", port, "left", port, 8)
r_out = u["rmux8_r"]

u = c.sub(lib["flags8"], "flags8", (1820, 760))
link(u, "flags8", "r", "left", "r", 8)
flag_ports = {f: u[f"flags8_{f}"] for f in ("n", "z")}

u = c.sub(lib["cmp8"], "cmp8", (2100, 760))
link(u, "cmp8", "a", "left", "a", 8)
link(u, "cmp8", "b", "left", "b", 8)
flag_ports.update({f: u[f"cmp8_{f}"] for f in ("s", "e", "f")})

# ---- inputs, left: op also split into its bits ------------------------------------
X_IN = 220
c.wire(c.input((X_IN, 100), f"{M}_op", 4), (X_IN + 80, 100), c.tunnel((X_IN + 100, 100), f"{M}_op", 4))
# op2 has no effect on arithmetic codes (01xx = 00xx); only rmux8 reads it, from alu_op
sp = c.splitter((X_IN + 40, 140), 4, [[0], [1], [3]], facing="east", appear="right", spacing=3)
c.wire((X_IN + 40, 100), (X_IN + 40, 140))
for k, bit in enumerate((0, 1, 3)):
    e = sp[f"end{k}"]
    c.wire(e, (e[0] + 20, e[1]))
    c.tunnel((e[0] + 20, e[1]), f"{M}_op_{bit}")
for i, (n, w) in enumerate((("cin", 1), ("a", 8), ("b", 8))):
    y = 320 + 40 * i
    c.wire(c.input((X_IN, y), f"{M}_{n}", w), c.tunnel((X_IN + 20, y), f"{M}_{n}", w))

# ---- result, right; flags, bottom ---------------------------------------------------
c.wire(r_out, (r_out[0] + 40, r_out[1]))
c.tunnel((r_out[0] + 40, r_out[1]), f"{M}_r", 8)
c.wire(c.tunnel((2400, r_out[1]), f"{M}_r", 8, facing="east"), c.output((2440, r_out[1]), f"{M}_r", 8))

Y_FLAGS = 1120
c.wire(c_out, c.output((c_out[0], Y_FLAGS), f"{M}_c", facing="north"))
for f, at in flag_ports.items():
    c.wire(at, c.output((at[0], Y_FLAGS), f"{M}_{f}", facing="north"))

c.symbol(top=[f"{M}_op", f"{M}_cin"], left=[f"{M}_a", f"{M}_b"], right=[f"{M}_r"],
         bottom=[f"{M}_c", f"{M}_z", f"{M}_n", f"{M}_s", f"{M}_e", f"{M}_f"])

if __name__ == "__main__":
    bad = p.save(OUT)
    import bom      # the bill of materials follows the circuits (notes/bom-alu.csv)
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-alu.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-alu.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import render
        import netlist
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M)
    sys.exit(1 if bad else 0)
