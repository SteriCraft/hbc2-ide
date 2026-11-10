#!/usr/bin/env python3
"""Check that a KiCad schematic wires its chips like a Logisim circuit.

Both sides are reduced to nets of chip pins; drawing positions are ignored.

* Logisim: the flattened bit-level netlist from netlist.py (--flat).
* KiCad: the netlist kicad-cli exports from the .kicad_sch.

Only chips are compared. A KiCad part is a chip when its value names a part
type used in the Logisim circuit (74HCT11 = 7411). Capacitors, diodes and
connectors are board extras and are left out, except that every Logisim
top-level pin must reach a connector (J) on the KiCad net of the same name.

Chips are matched by type, so references don't need to agree: for each type
the script picks the pairing that puts the most pins on nets of the same
name, then tries swaps that reduce the differences. Each chip pin is then
compared by the set of chip pins it is wired to.

Net names are compared loosely: case, a leading "/", "[k]" vs "_k" vs "k"
are ignored, VCC = +5V, and a Logisim pair X_in_k / X_out_k (a bidirectional
bus split for simulation) is one net X_k.

Usage:
    python kicadcmp.py circuits/flags/flags.circ ../kicad/flags/hbc2_flags.kicad_sch
    python kicadcmp.py circuits/alu/alu.circ ../kicad/alu/hbc2_alu.kicad_sch --top alu

Exit status 0 when the two agree, 1 when they differ.
"""

from __future__ import annotations

import argparse
import itertools
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist  # noqa: E402

# ---------------------------------------------------------------- names


def canon(name: str) -> str:
    """Loose net name: flags_op[0], flags_op_0, /flags_op0 -> flags_op0.
    The Logisim subcircuit path (add_sub8@1000,300/) and the KiCad sheet path
    are dropped."""
    n = name.strip().split("/")[-1].lower()
    n = re.sub(r"\[(\d+)\]$", r"_\1", n)
    n = re.sub(r"_(\d+)$", r"\1", n)
    return {"+5v": "vcc", "5v": "vcc", "vdd": "vcc", "gnd": "gnd", "vss": "gnd"}.get(n, n)


def auto_name(name: str) -> bool:
    """Names a tool made up (Logisim N12, KiCad Net-(...), unconnected-(...))."""
    return bool(re.fullmatch(r"N\d+", name)) or name.startswith(("Net-(", "unconnected-("))


def _quad2(): return [[(1, 2, 3), (4, 5, 6), (9, 10, 8), (12, 13, 11)]]
def _tri3(): return [[(1, 2, 13, 12), (3, 4, 5, 6), (9, 10, 11, 8)]]


# Pins a board may legitimately swap. "units": groups of interchangeable units
# (gates, buffers, flip-flops, mux sections sharing their control pins), each
# unit a tuple of DIP pins in the same roles. "commute": pins of one unit that
# can trade places (inputs of an AND gate, A/B of one adder bit).
SWAPS = {
    "7400": {"units": _quad2()}, "7408": {"units": _quad2()},
    "7432": {"units": _quad2()}, "7486": {"units": _quad2()},
    "7402": {"units": [[(2, 3, 1), (5, 6, 4), (8, 9, 10), (11, 12, 13)]]},
    "7410": {"units": _tri3()}, "7411": {"units": _tri3()}, "7427": {"units": _tri3()},
    "7404": {"units": [[(1, 2), (3, 4), (5, 6), (9, 8), (11, 10), (13, 12)]]},
    "7414": {"units": [[(1, 2), (3, 4), (5, 6), (9, 8), (11, 10), (13, 12)]]},
    "74541": {"units": [[(2 + k, 18 - k) for k in range(8)]]},
    "74540": {"units": [[(2 + k, 18 - k) for k in range(8)]]},
    "74245": {"units": [[(2 + k, 18 - k) for k in range(8)]]},
    "74574": {"units": [[(2 + k, 19 - k) for k in range(8)]]},
    "74240": {"units": [[(2, 18), (4, 16), (6, 14), (8, 12)], [(11, 9), (13, 7), (15, 5), (17, 3)]]},
    "74244": {"units": [[(2, 18), (4, 16), (6, 14), (8, 12)], [(11, 9), (13, 7), (15, 5), (17, 3)]]},
    "74377": {"units": [[(3, 2), (4, 5), (7, 6), (8, 9), (13, 12), (14, 15), (17, 16), (18, 19)]]},
    "74175": {"units": [[(4, 2, 3), (5, 7, 6), (12, 10, 11), (13, 15, 14)]]},
    "74157": {"units": [[(2, 3, 4), (5, 6, 7), (11, 10, 9), (14, 13, 12)]]},
    "74153": {"units": [[(1, 6, 5, 4, 3, 7), (15, 10, 11, 12, 13, 9)]]},
    "74283": {"commute": [(5, 6), (3, 2), (14, 15), (12, 11)]},
}
# Inputs of a gate commute: every pin of a "units" gate but the last (output).
for _t in ("7400", "7408", "7432", "7486", "7402", "7410", "7411", "7427"):
    SWAPS[_t]["commute"] = [u[:-1] for g in SWAPS[_t]["units"] for u in g]


def chip_type(value: str) -> str:
    """74HCT11 -> 7411, 74ACT574 -> 74574; other parts lower-cased."""
    v = value.strip()
    m = re.match(r"^(?:SN)?74[A-Z]*?(\d+)", v, re.I)
    return "74" + m.group(1) if m else v.lower()


# ---------------------------------------------------------------- loading

class Side:
    """Nets of one design: name -> set of (ref, pin), plus chip types."""

    def __init__(self, label: str):
        self.label = label
        self.nets: dict[str, set] = {}
        self.types: dict[str, str] = {}      # chip ref -> type
        self.pin_names: dict[tuple, str] = {}
        self.io: dict[str, set] = {}         # net name -> set of I/O refs (pins or connectors)

    def net_of(self) -> dict:
        return {t: n for n, ts in self.nets.items() for t in ts}


def load_logisim(path: Path, top: str | None) -> Side:
    proj = netlist.Project(path)
    top = top or proj.main
    if top not in proj.circuits:
        raise SystemExit(f"error: {path.name} has no circuit '{top}' "
                         f"(available: {', '.join(proj.circuits)})")
    d = netlist.flatten(proj.circuits[top])
    s = Side(f"Logisim {path.name} ({top})")
    for net in d["nets"]:
        chips, io = set(), set()
        for t in net["terminals"]:
            if t["type"] == "Pin":
                io.add(t["ref"])
            elif t.get("pin") is not None:          # sim-only pins have no number
                chips.add((t["ref"], str(t["pin"])))
                s.types[t["ref"]] = t["type"]
                s.pin_names[(t["ref"], str(t["pin"]))] = t.get("name", "")
        s.nets[net["name"]] = chips
        if io:
            s.io[net["name"]] = io
    # A bidirectional bus split for simulation: X_in_k + X_out_k -> X_k.
    for name in list(s.nets):
        m = re.fullmatch(r"(.*)_in(_?\d+)", name)
        if m:
            other = f"{m.group(1)}_out{m.group(2)}"
            if other in s.nets:
                joined = f"{m.group(1)}{m.group(2)}"
                s.nets[joined] = s.nets.pop(name) | s.nets.pop(other)
                s.io[joined] = s.io.pop(name, set()) | s.io.pop(other, set())
    return s


def find_kicad_cli() -> str:
    exe = shutil.which("kicad-cli")
    if exe:
        return exe
    roots = [Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "KiCad"]
    found = sorted(p for r in roots if r.exists() for p in r.glob("*/bin/kicad-cli.exe"))
    if found:
        return str(found[-1])
    raise SystemExit("error: kicad-cli not found (install KiCad or put kicad-cli on PATH)")


def load_kicad(path: Path, logisim_types: set) -> tuple[Side, dict]:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "board.net"
        r = subprocess.run([find_kicad_cli(), "sch", "export", "netlist", "-o", str(out), str(path)],
                           capture_output=True, text=True)
        if r.returncode or not out.exists():
            raise SystemExit(f"error: kicad-cli failed on {path}:\n{r.stdout}{r.stderr}")
        text = out.read_text(encoding="utf-8")

    values = dict(re.findall(r'\(comp\s+\(ref "([^"]+)"\)\s+\(value "([^"]*)"\)', text))
    s = Side(f"KiCad {path.name}")
    extras = {}
    for ref, val in values.items():
        t = chip_type(val)
        if t in logisim_types:
            s.types[ref] = t
        else:
            extras[ref] = val
    body = text[text.index("(nets"):]
    for block in re.split(r"\(net\s*\(code", body)[1:]:
        name = re.search(r'\(name "([^"]*)"\)', block).group(1)
        chips, io = set(), set()
        for node in re.finditer(r'\(node\s+\(ref "([^"]+)"\)\s+\(pin "([^"]+)"\)(?:\s+\(pinfunction "([^"]*)"\))?',
                                block):
            ref, pin, fn = node.groups()
            if ref in s.types:
                chips.add((ref, pin))
                s.pin_names[(ref, pin)] = fn or ""
            elif ref.upper().startswith("J"):
                io.add(f"{ref}.{pin}")
        s.nets[name] = chips
        if io:
            s.io[name] = io
    return s, extras


# ---------------------------------------------------------------- matching

class Match:
    """Logisim chip -> KiCad chip, and per chip Logisim pin -> KiCad pin."""

    def __init__(self, lg: Side, kc: Side):
        self.lg, self.kc = lg, kc
        self.chip: dict[str, str] = {}
        self.pin: dict[str, dict] = defaultdict(dict)

    def k(self, t):
        """KiCad terminal of a Logisim terminal (None if its chip is unmatched)."""
        ref, pin = t
        if ref not in self.chip:
            return None
        return self.chip[ref], self.pin[ref].get(pin, pin)

    def l_pin(self, kref, kpin):
        """Logisim pin name of a KiCad terminal, for display."""
        back = {v: l for l, v in self.chip.items()}
        lref = back.get(kref)
        if lref is None:
            return self.kc.pin_names.get((kref, kpin), "")
        inv = {v: q for q, v in self.pin[lref].items()}
        return self.lg.pin_names.get((lref, inv.get(kpin, kpin)), "")


def pin_names_by_ref(side: Side) -> dict:
    """ref -> {pin: canonical net name} for named nets only."""
    out = defaultdict(dict)
    for name, ts in side.nets.items():
        if not auto_name(name):
            for ref, pin in ts:
                out[ref][pin] = canon(name)
    return out


def moves(m: Match, lref: str):
    """Allowed pin swaps on one chip, each a list of (Logisim pin, Logisim pin)."""
    sw = SWAPS.get(m.lg.types[lref], {})
    for group in sw.get("units", []):
        for u, v in itertools.combinations(group, 2):
            yield [(str(x), str(y)) for x, y in zip(u, v)]
    for comm in sw.get("commute", []):
        for x, y in itertools.combinations(comm, 2):
            yield [(str(x), str(y))]


def apply(m: Match, lref: str, move):
    pm = m.pin[lref]
    for x, y in move:
        pm[x], pm[y] = pm.get(y, y), pm.get(x, x)


def match(lg: Side, kc: Side) -> Match:
    m = Match(lg, kc)
    ln, kn = pin_names_by_ref(lg), pin_names_by_ref(kc)

    def score(a, b):
        return sum(1 for p, n in ln[a].items() if kn[b].get(p) == n)

    for typ in sorted(set(lg.types.values())):
        ls = sorted(r for r, t in lg.types.items() if t == typ)
        ks = sorted(r for r, t in kc.types.items() if t == typ)
        n = min(len(ls), len(ks))
        best, best_s = [], -1
        if n <= 6:
            for lsub in itertools.combinations(ls, n):
                for perm in itertools.permutations(ks, n):
                    sc = sum(score(a, b) for a, b in zip(lsub, perm))
                    if sc > best_s:
                        best, best_s = list(zip(lsub, perm)), sc
        else:                                   # greedy for large groups
            pairs = sorted(((score(a, b), a, b) for a in ls for b in ks), reverse=True)
            used = set()
            for _, a, b in pairs:
                if a not in used and b not in used:
                    best.append((a, b)); used |= {a, b}
        m.chip.update(best)

    # Hill-climb: chip swaps within a type, then pin swaps within a chip,
    # kept whenever they reduce the number of differing pins.
    cost = len(compare(m)[0])
    improved = True
    while improved and cost:
        improved = False
        bad = compare(m)[0]
        back = {v: l for l, v in m.chip.items()}
        suspects = sorted({back[r] for r, _ in bad if r in back})
        for a, b in itertools.combinations(suspects, 2):
            if lg.types[a] == lg.types[b]:
                m.chip[a], m.chip[b] = m.chip[b], m.chip[a]
                m.pin[a], m.pin[b] = m.pin[b], m.pin[a]
                c = len(compare(m)[0])
                if c < cost:
                    cost, improved = c, True
                    break
                m.chip[a], m.chip[b] = m.chip[b], m.chip[a]
                m.pin[a], m.pin[b] = m.pin[b], m.pin[a]
        if improved:
            continue
        for lref in suspects:
            for mv in list(moves(m, lref)):
                apply(m, lref, mv)
                c = len(compare(m)[0])
                if c < cost:
                    cost, improved = c, True
                else:
                    apply(m, lref, mv)
    return m


# ---------------------------------------------------------------- comparing

def compare(m: Match):
    """KiCad terminals whose peers differ, and the two terminal -> peers maps."""
    def peers(side, translate):
        out = {}
        for ts in side.nets.values():
            group = frozenset(x for x in (translate(t) for t in ts) if x)
            for t in group:
                out[t] = group
        return out

    matched_k = set(m.chip.values())
    lp = peers(m.lg, m.k)
    kp = peers(m.kc, lambda t: t if t[0] in matched_k else None)
    bad = {t for t in set(lp) | set(kp)
           if lp.get(t, frozenset({t})) != kp.get(t, frozenset({t}))}
    return bad, lp, kp


def refkey(r: str):
    return (re.sub(r"\d", "", r), int(re.sub(r"\D", "", r) or 0))


def report(lg: Side, kc: Side, extras: dict) -> int:
    m = match(lg, kc)
    back = {k: l for l, k in m.chip.items()}
    problems = 0
    print(f"{lg.label}\n  vs {kc.label}\n")

    # Chips
    print(f"Chips: {len(m.chip)} matched")
    for l in sorted(m.chip, key=lambda r: (lg.types[r], refkey(m.chip[r]))):
        print(f"  {lg.types[l]:<8} KiCad {m.chip[l]:<5} = Logisim {l}")
    for r in sorted(set(lg.types) - set(m.chip)):
        print(f"  ! Logisim {r} ({lg.types[r]}) has no KiCad counterpart"); problems += 1
    for r in sorted(set(kc.types) - set(back), key=refkey):
        print(f"  ! KiCad {r} ({kc.types[r]}) has no Logisim counterpart"); problems += 1
    if extras:
        print(f"  board extras, not compared: {', '.join(sorted(extras, key=refkey))}")

    # Gates / units the board uses differently (same function).
    swaps = []
    for l, pm in m.pin.items():
        moved = sorted(((q, v) for q, v in pm.items() if q != v), key=lambda x: int(x[0]))
        if moved:
            swaps.append((refkey(m.chip[l]), f"  {m.chip[l]} ({lg.types[l]}): " + ", ".join(
                f"Logisim pin {q}({lg.pin_names.get((l, q), '')}) is KiCad pin {v}" for q, v in moved)))
    if swaps:
        print("\nSwapped gates or inputs, same function (not an error):")
        print("\n".join(s for _, s in sorted(swaps)))

    # Nets
    bad, lp, kp = compare(m)
    lnet = {m.k(t): n for n, ts in lg.nets.items() for t in ts if m.k(t)}
    knet = kc.net_of()

    def show(t):
        nm = m.l_pin(*t)
        return f"{t[0]}.{t[1]}" + (f"({nm})" if nm else "")

    def key(t):
        return refkey(t[0]) + (int(t[1]) if t[1].isdigit() else 0,)

    print()
    if not bad:
        print("Nets: every chip pin has the same connections on both sides.")
    else:
        print("Nets wired differently (KiCad references, Logisim pin names):")
        seen, groups = set(), 0
        for t in sorted(bad, key=key):
            if t in seen:
                continue
            lgrp, kgrp = lp.get(t, frozenset({t})), kp.get(t, frozenset({t}))
            seen |= lgrp | kgrp
            groups += 1
            print(f"\n  around {show(t)}  Logisim net {lnet.get(t, '(unconnected)')}, "
                  f"KiCad net {knet.get(t, '(unconnected)')}")
            if lgrp - kgrp:
                print("    only in Logisim:", ", ".join(show(x) for x in sorted(lgrp - kgrp, key=key)))
            if kgrp - lgrp:
                print("    only in KiCad:  ", ", ".join(show(x) for x in sorted(kgrp - lgrp, key=key)))
            both = sorted(lgrp & kgrp, key=key)
            if len(both) > 12:
                print(f"    both:             {len(both)} pins")
            else:
                print("    both:            ", ", ".join(show(x) for x in both) or "-")
        problems += groups

    # Same pins, different names (only when both are real names).
    renamed = set()
    for name, ts in lg.nets.items():
        names = {canon(name)} | {canon(p) for p in lg.io.get(name, ())}
        kts = [m.k(t) for t in ts if m.k(t)]
        if auto_name(name) or not kts or kts[0] in bad:
            continue
        kname = knet.get(kts[0])
        if kname and not auto_name(kname) and canon(kname) not in names:
            renamed.add((name, kname))
    if renamed:
        print("\nSame wiring, different net names (not an error):")
        for a, b in sorted(renamed):
            print(f"  Logisim {a}  =  KiCad {b}")

    # Top-level pins must reach a connector on the matching KiCad net.
    reached, missing = set(), []
    kio_by_canon = {canon(n): n for n in kc.io}
    for name, pins in lg.io.items():
        if canon(name) in ("vcc", "gnd"):
            continue
        kts = [m.k(t) for t in lg.nets[name] if m.k(t)]
        kname = knet.get(kts[0]) if kts else kio_by_canon.get(canon(name))
        if kname in kc.io:
            reached.add(kname)
        else:
            missing.append((name, pins, kname))
    if missing:
        print("\nLogisim top-level pins whose KiCad net has no connector:")
        for name, pins, kname in sorted(missing):
            print(f"  {', '.join(sorted(pins))}  (KiCad net {kname or 'not found'})")
        problems += len(missing)
    extra_io = sorted(n for n in kc.io if n not in reached and canon(n) not in ("vcc", "gnd"))
    if extra_io:
        print("\nKiCad connector nets with no Logisim top-level pin (check they are meant):")
        for n in extra_io:
            print(f"  {n}  ({', '.join(sorted(kc.io[n]))})")

    print("\nResult:", "EQUIVALENT" if not problems else f"{problems} difference(s)")
    return 1 if problems else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Compare a KiCad schematic's chip wiring with a Logisim circuit.")
    ap.add_argument("circ", help="Logisim .circ file (looked up in logisim/circuits too)")
    ap.add_argument("kicad_sch", help="KiCad .kicad_sch file")
    ap.add_argument("--top", help="Logisim circuit to flatten (default: the file's main circuit)")
    args = ap.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    lg = load_logisim(netlist.resolve_file(args.circ), args.top)
    kc, extras = load_kicad(Path(args.kicad_sch), set(lg.types.values()))
    sys.exit(report(lg, kc, extras))


if __name__ == "__main__":
    main()
