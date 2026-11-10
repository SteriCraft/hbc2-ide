#!/usr/bin/env python3
"""Rig maps: which test-rig bit is wired to which card signal (notes/test-rig.md).

The test rig is a generic chain: D0..D63 (eight 74HCT595, drive the card's
inputs, one /OE per chip), S0..S63 (eight 74HCT165, read the card's outputs)
and C0..C7 (Mega pins, clocks, set after the D bits). A map file per card,
logisim/rig/<card>.map, says which rig bit goes to which bit of the card
circuit's pins, so the replay tool can apply `hbc2 vectors` rows to the card.
The card's connector pinout is not needed: the optional third column
records it once it is decided.

Map file, one wire per line, `#` starts a comment:

    <rig>       <pins>                              [<conn>]
    D3          alu_op[3]
    S0          alu_n
    C0          clk
    VCC         some_input                          (tied on the bench)
    D56+S8      flags_bus_in[0]+flags_bus_out[0]    J1.17

`name[k]` is bit k of a multi-bit pin, a bare name is a 1-bit pin. `+`
joins the two sides of one physical wire (a bus the card both reads and
drives, split into an input pin and an output pin in Logisim): one D bit
and one S bit, one input pin bit and one output pin bit. The rig drives the
wire only while the card doesn't (notes/test-rig.md, "Replay"), so a 595
carrying such a wire carries only that bus.

Usage:
    python rigmap.py gen flags            # write rig/flags.map (refuses to overwrite; --force)
    python rigmap.py check flags          # exit 1 on an error
    python rigmap.py check flags --vectors flags.txt   # also the vector header
    python rigmap.py all                  # check every card that has a map, print the rig usage
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist  # noqa: E402

LOGISIM = Path(__file__).resolve().parent.parent
RIG_DIR = LOGISIM / "rig"

# The rig as built (notes/test-rig.md). Change these if a chain gets longer.
RIG = {"D": 64, "S": 64, "C": 8}
CHIP = 8   # bits per 595 / 165

# card -> (circuit file, circuit, where its vectors come from)
CARDS = {
    "clkgen": ("circuits/cu/clkgen.circ", "clkgen", "designs/clkgen.py --test (sequences in Python)"),
    "ir": ("circuits/ir/ir.circ", "ir", "hbc2 vectors ir"),
    "alu": ("circuits/alu/alu.circ", "alu", "hbc2 vectors alu"),
    "flags": ("circuits/flags/flags.circ", "flags", "hbc2 vectors flags"),
    "regs": ("circuits/registers/regs.circ", "regs", "hbc2 vectors regs"),
    "addr": ("circuits/addr/addr.circ", "addr", "hbc2 vectors addr"),
    "mem": ("circuits/mem/mem.circ", "mem", "hbc2 vectors mem"),
    "cu": ("circuits/cu/cu.circ", "cu", "hbc2 vectors cu"),
}


def is_clock(name: str) -> bool:
    """Inputs the rig gives a Mega pin (C bits): set after the data, like an edge."""
    return name == "clk" or name.startswith("clk_") or name.endswith("_osc")


def card_pins(card: str):
    """[(name, width, 'in'|'out')] of the card circuit, in file order."""
    path, circ, _ = CARDS[card]
    proj = netlist.Project(LOGISIM / path)
    c = proj.circuits[circ]
    out = []
    for p in c.pins():
        width = netlist.parse_int(p.attrs.get("width"), 1)
        out.append((p.label, width, "out" if p.attrs.get("type") == "output" else "in"))
    return out


def bits(name: str, width: int) -> list[str]:
    return [name] if width == 1 else [f"{name}[{k}]" for k in range(width)]


def partner(name: str, outputs: dict) -> str | None:
    """The output pin on the same wire as input `name` (x_in <-> x / x_out)."""
    if not name.endswith("_in"):
        return None
    stem = name[:-3]
    for cand in (stem, stem + "_out"):
        if cand in outputs:
            return cand
    return None


# ---------------------------------------------------------------- gen

def generate(card: str) -> str:
    pins = card_pins(card)
    inputs = [(n, w) for n, w, d in pins if d == "in"]
    outputs = {n: w for n, w, d in pins if d == "out"}
    pairs = {n: partner(n, outputs) for n, _ in inputs}
    paired_out = {o for o in pairs.values() if o}

    lines = []
    c = d = s = 0
    for n, w in inputs:                      # clocks on Mega pins
        if is_clock(n):
            for b in bits(n, w):
                lines.append((f"C{c}", b)); c += 1
    for n, w in inputs:                      # plain inputs, packed from D0
        if not is_clock(n) and not pairs[n]:
            for b in bits(n, w):
                lines.append((f"D{d}", b)); d += 1
    for n, w in outputs.items():             # plain outputs, packed from S0
        if n not in paired_out:
            for b in bits(n, w):
                lines.append((f"S{s}", b)); s += 1
    for n, w in inputs:                      # buses: each on its own 595(s)
        o = pairs[n]
        if not o:
            continue
        if outputs[o] != w:
            raise SystemExit(f"error: {n} ({w} bits) and {o} ({outputs[o]} bits) differ in width")
        d = -(-d // CHIP) * CHIP
        for bi, bo in zip(bits(n, w), bits(o, w)):
            lines.append((f"D{d}+S{s}", f"{bi}+{bo}")); d += 1; s += 1

    path, circ, vec = CARDS[card]
    head = [f"# Rig map of card {card}: {path} circuit {circ}; vectors: {vec}",
            "# Generated by tools/rigmap.py gen, then edited by hand if needed (notes/test-rig.md).",
            "# Third column: the card connector pin, once the pinout is decided.",
            "#",
            "# rig         pins"]
    body = [f"{r:<13} {p}" for r, p in lines]
    return "\n".join(head + body) + "\n"


# ---------------------------------------------------------------- check

RIG_RE = re.compile(r"^([DSC])(\d+)$")
PIN_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)(?:\[(\d+)\])?$")


class Map:
    def __init__(self, path: Path):
        self.path = path
        self.wires = []   # (lineno, [rig bits], [pin bits], conn)
        for no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            line = raw.split("#", 1)[0].split()
            if not line:
                continue
            if len(line) not in (2, 3):
                raise SystemExit(f"{path}:{no}: expected '<rig> <pins> [<conn>]'")
            self.wires.append((no, line[0].split("+"), line[1].split("+"), line[2] if len(line) == 3 else None))


def check(card: str, map_path: Path, vectors: Path | None = None, quiet=False) -> tuple[int, dict]:
    pins = card_pins(card)
    width = {n: w for n, w, _ in pins}
    dirn = {n: d for n, w, d in pins}
    m = Map(map_path)
    errors: list[str] = []
    warns: list[str] = []
    used_rig: dict[str, int] = {}
    used_pin: dict[str, int] = {}
    used_conn: dict[str, int] = {}
    chip_owner: dict[int, set] = {}     # D chip -> {"plain"} or {bus input pin name}
    usage = {"D": set(), "S": set(), "C": set()}

    def err(no, msg):
        errors.append(f"{map_path.name}:{no}: {msg}")

    for no, rigs, pbits, conn in m.wires:
        # pins
        parsed = []
        for pb in pbits:
            mm = PIN_RE.match(pb)
            if not mm or mm.group(1) not in width:
                err(no, f"unknown pin {pb!r}"); continue
            name, k = mm.group(1), mm.group(2)
            w = width[name]
            if (k is None) != (w == 1) or (k is not None and int(k) >= w):
                err(no, f"{pb!r}: {name} is {w} bit(s) wide"); continue
            if pb in used_pin:
                err(no, f"{pb} already mapped on line {used_pin[pb]}")
            used_pin[pb] = no
            parsed.append((name, dirn[name]))
        # rig bits
        kinds = []
        for r in rigs:
            if r in ("VCC", "GND"):
                kinds.append(r); continue
            mm = RIG_RE.match(r)
            if not mm or int(mm.group(2)) >= RIG[mm.group(1)]:
                err(no, f"no rig bit {r!r} (rig: D0-D{RIG['D']-1}, S0-S{RIG['S']-1}, C0-C{RIG['C']-1}, VCC, GND)")
                continue
            if r in used_rig:
                err(no, f"rig bit {r} already used on line {used_rig[r]}")
            used_rig[r] = no
            kinds.append(mm.group(1))
            usage[mm.group(1)].add(int(mm.group(2)))
        if conn:
            if conn in used_conn:
                err(no, f"connector pin {conn} already used on line {used_conn[conn]}")
            used_conn[conn] = no
        if len(parsed) != len(pbits) or len(kinds) != len(rigs):
            continue
        # direction rules
        if len(rigs) == 1 and len(pbits) == 1:
            k, (name, d) = kinds[0], parsed[0]
            if k in ("D", "C", "VCC", "GND") and d != "in":
                err(no, f"{k} drives a card input; {name} is an output")
            if k == "S" and d != "out":
                err(no, f"S reads a card output; {name} is an input")
            if k == "D":
                chip_owner.setdefault(int(rigs[0][1:]) // CHIP, set()).add("plain")
        elif len(rigs) == 2 and len(pbits) == 2:
            if sorted(kinds) != ["D", "S"] or sorted(d for _, d in parsed) != ["in", "out"]:
                err(no, "a joined wire is one D bit + one S bit, one input pin bit + one output pin bit")
                continue
            dbit = next(r for r in rigs if r.startswith("D"))
            inp = next(n for n, d in parsed if d == "in")
            chip_owner.setdefault(int(dbit[1:]) // CHIP, set()).add(inp)
        else:
            err(no, "a line is one rig bit and one pin bit, or a joined wire (D+S, in+out)")

    for chip, owners in sorted(chip_owner.items()):
        if len(owners) > 1:
            errors.append(f"{map_path.name}: 595 #{chip} (D{chip*CHIP}-D{chip*CHIP+7}) mixes "
                          f"{', '.join(sorted(owners))}: a bus needs its 595s to itself (its /OE goes off)")

    for name, w, d in pins:
        for b in bits(name, w):
            if b not in used_pin:
                (errors if d == "in" else warns).append(
                    f"{map_path.name}: {d}put {b} not mapped" + ("" if d == "in" else " (not checked by the replay)"))

    if vectors:
        header = vectors.read_text(encoding="utf-8").splitlines()[0].split()
        for col in header:
            if col in ("<set>", "<seq>"):
                continue
            mm = re.match(r"^([A-Za-z_][A-Za-z0-9_]*)(?:\[(\d+)\])?$", col)
            name, w = mm.group(1), int(mm.group(2) or 1)
            if name not in width:
                errors.append(f"{vectors.name}: column {col} is not a pin of the card")
            elif width[name] != w:
                errors.append(f"{vectors.name}: column {col}, the card's pin is {width[name]} bits")

    if not quiet:
        for w in warns:
            print("warning:", w)
    for e in errors:
        print("error:", e)
    return len(errors), usage


def usage_line(card, u):
    """Bits used per chain, and the chips a chain must have (up to its highest bit)."""
    def chips(k):
        return -(-(max(u[k]) + 1) // CHIP) if u[k] else 0
    return (f"{card:<8} D {len(u['D']):>2} bits, {chips('D')} x 595   "
            f"S {len(u['S']):>2} bits, {chips('S')} x 165   C {len(u['C'])}")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gen"); g.add_argument("card", choices=CARDS); g.add_argument("--force", action="store_true")
    g.add_argument("-o", "--output")
    c = sub.add_parser("check"); c.add_argument("card"); c.add_argument("--vectors")
    sub.add_parser("all")
    a = ap.parse_args(argv)

    if a.cmd == "gen":
        out = Path(a.output) if a.output else RIG_DIR / f"{a.card}.map"
        if out.exists() and not a.force:
            raise SystemExit(f"error: {out} exists (edited by hand?); use --force to overwrite")
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(generate(a.card), encoding="utf-8", newline="\n")
        n, u = check(a.card, out, quiet=True)
        print(f"wrote {out}: {usage_line(a.card, u)}")
        return 1 if n else 0

    if a.cmd == "check":
        p = Path(a.card)
        card = p.stem if p.suffix == ".map" else a.card
        path = p if p.suffix == ".map" else RIG_DIR / f"{card}.map"
        if card not in CARDS:
            raise SystemExit(f"error: unknown card {card} ({', '.join(CARDS)})")
        n, u = check(card, path, Path(a.vectors) if a.vectors else None)
        print(("FAIL " if n else "ok   ") + usage_line(card, u))
        return 1 if n else 0

    bad = 0
    for card in CARDS:
        path = RIG_DIR / f"{card}.map"
        if not path.exists():
            print(f"{card:<8} no map")
            continue
        n, u = check(card, path, quiet=True)
        bad += n
        print(("FAIL " if n else "ok   ") + usage_line(card, u))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
