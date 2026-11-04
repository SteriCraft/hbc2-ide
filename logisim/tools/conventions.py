#!/usr/bin/env python3
"""Check a Logisim-evolution file against the HBC-2 design conventions
(logisim/notes/conventions.md), and summarise each module.

Circuit roles come from the name:
  main        only connects modules: subcircuits and wiring parts
  test_<m>    test bench: any component, no naming rules
  other       a module: #TTL chips, RAM/ROM, wiring parts, subcircuits

Checks:
  errors    forbidden component in a module or in main, unlabelled pin,
            pin/tunnel name not snake_case or without the module prefix,
            chip label not U<n> or used twice, floating chip input (from
            circcheck)
  warnings  floating subcircuit input (work in progress), chips not
            numbered top to bottom then left to right, splitter end
            carrying non-contiguous bits, name ending in the bus width,
            test bench whose module does not exist

Constant is allowed in modules: it stands for inputs tied to GND/5V.

Usage:
    python conventions.py add8_ttl.circ
    python conventions.py cpu.circ -c add_sub8
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import circcheck  # noqa: E402
import netlist as nl  # noqa: E402

SNAKE = re.compile(r"^[a-z][a-z0-9]*(_[a-z0-9]+)*$")
GLOBAL = re.compile(r"^(clk|rst_n|pc|sp|reg_[a-z]|flag_[a-z]|bus_[a-z0-9_]+)$")
WIRING_OK = {"Pin", "Splitter", "Tunnel", "Probe", "Clock", "Ground", "Power"}
MODULE_WIRING_OK = WIRING_OK | {"Constant"}   # tied to GND/5V, e.g. 01 for INC
MEMORY_OK = {"RAM", "ROM"}


def role(name: str) -> str:
    return "main" if name == "main" else "test" if name.startswith("test_") else "module"


def _comps(circ: nl.Circuit):
    """(lib, name, loc, attrs) of every component, Text included."""
    libs = circ.project.libs
    for e in circ.elem.findall("comp"):
        lib = libs.get(e.get("lib"), "") if e.get("lib") is not None else ""
        yield lib, e.get("name"), nl.parse_loc(e.get("loc")), nl.read_attrs(e)


def _allowed(kind: str, lib: str, name: str, circ: nl.Circuit) -> bool:
    if lib == "" or lib.startswith("file#"):
        return True                                   # subcircuit
    if lib == "#Wiring":
        return name in (MODULE_WIRING_OK if kind == "module" else WIRING_OK)
    if lib == "#Base":
        return name == "Text"
    if kind == "main":
        return False
    return lib == "#TTL" or (lib == "#Memory" and name in MEMORY_OK)


def check_circuit(circ: nl.Circuit) -> tuple[list[circcheck.Issue], dict]:
    kind = role(circ.name)
    issues: list[circcheck.Issue] = []

    def add(level, where, text):
        issues.append(circcheck.Issue(level, circ.name, where, text))

    if not SNAKE.match(circ.name):
        add("error", (0, 0), f"circuit name {circ.name!r} is not lowercase snake_case")
    if kind == "test" and circ.name[5:] not in circ.project.circuits:
        add("warning", (0, 0), f"test bench {circ.name}: no module named {circ.name[5:]!r}")

    chips, ports = [], []
    for lib, name, loc, a in _comps(circ):
        if kind != "test" and not _allowed(kind, lib, name, circ):
            add("error", loc, f"{name} ({lib}) at {loc} is not allowed in {'main' if kind == 'main' else 'a module'}")
        if lib == "#TTL":
            chips.append((name, loc, a.get("label", ""), a.get("facing", "east")))
        if kind == "test" or lib != "#Wiring" or name not in ("Pin", "Tunnel"):
            continue
        label, width = a.get("label", ""), nl.parse_int(a.get("width"), 1)
        if name == "Pin":
            ports.append((a.get("type") == "output", label, width))
        if not label:
            if name == "Pin":
                add("error", loc, f"pin at {loc} has no label")
            continue
        what = f"{name.lower()} {label!r}"
        if not SNAKE.match(label):
            add("error", loc, f"{what} is not lowercase snake_case")
        elif kind == "module" and not label.startswith(circ.name + "_") and not GLOBAL.match(label):
            add("error", loc, f"{what} lacks the module prefix '{circ.name}_'")
        if width > 1 and label.endswith(str(width)):
            add("warning", loc, f"{what} seems to carry its width ({width}) in its name")

    # chip labels
    seen = Counter(lab for _, _, lab, _ in chips)
    for name, loc, lab, _ in chips:
        if not re.fullmatch(r"U[1-9][0-9]*", lab):
            add("error", loc, f"{name} at {loc}: label {lab!r} is not U<n>")
        elif seen[lab] > 1:
            add("error", loc, f"{name} at {loc}: label {lab} is used {seen[lab]} times")
    def body(chip):         # (top, left) of the chip body; the anchor depends on the facing
        name, loc, _, facing = chip
        if name not in nl.TTL:
            return loc[1], loc[0]
        x1, y1, _, _ = circcheck.ttl_body(nl.Comp("ttl", name, "#TTL", loc, {"facing": facing}))
        return y1, x1
    order = [c[2] for c in sorted(chips, key=body)]
    expect = [f"U{i}" for i in range(1, len(chips) + 1)]
    if chips and all(re.fullmatch(r"U\d+", x) for x in order) and order != expect:
        add("warning", (0, 0), f"chips are not numbered top to bottom, then left to right: "
                               f"{' '.join(order)} instead of {' '.join(expect)}")

    # splitters: one group of contiguous bits per end
    circ.build()
    for c in circ.comps:
        if c.kind != "splitter":
            continue
        for k in range(len(c.ports) - 1):
            bits = [b for b, e in enumerate(c.bit_end) if e == k]
            if bits and bits != list(range(bits[0], bits[-1] + 1)):
                add("warning", c.loc, f"splitter at {c.loc}: end {k} carries non-contiguous bits {bits}")

    # floating inputs: errors on chips, warnings on subcircuits (work in progress)
    subs = {p.loc for c in circ.comps if c.kind == "sub" for p in c.ports}
    wiring, _ = circcheck.check_circuit(circ)
    for i in wiring:
        if "floating" in i.text:
            level = "warning" if i.where in subs else "error"
            issues.append(circcheck.Issue(level, i.circuit, i.where, i.text))

    return issues, {"role": kind, "chips": Counter(c[0] for c in chips), "ports": ports}


def summary(circ: nl.Circuit, st: dict) -> list[str]:
    lines = []
    if st["chips"]:
        lines.append("chips: " + ", ".join(f"{n} x {p} (74HCT{p[2:]})" for p, n in sorted(st["chips"].items())))
    for out, title in ((False, "inputs"), (True, "outputs")):
        ps = [f"{lab or '?'}[{w}]" if w > 1 else (lab or "?") for o, lab, w in st["ports"] if o == out]
        if ps:
            lines.append(f"{title}: {', '.join(ps)}")
    return lines


def report(path, circuits=None) -> int:
    """Print the conventions check of one file. Returns the number of errors."""
    proj = nl.Project(Path(path))
    errors = 0
    for name, circ in proj.circuits.items():
        if circuits and name not in circuits:
            continue
        issues, st = check_circuit(circ)
        errs = sum(i.level == "error" for i in issues)
        print(f"{name} ({st['role']}): conventions -- {errs} errors, {len(issues) - errs} warnings")
        for line in summary(circ, st):
            print("  " + line)
        for i in sorted(issues, key=lambda i: (i.level != "error", i.where[1], i.where[0])):
            print(f"  {i.level:7} {i.text}")
        errors += errs
    return errors


def main(argv=None):
    ap = argparse.ArgumentParser(description="Check Logisim circuits against the HBC-2 conventions.")
    ap.add_argument("files", nargs="+")
    ap.add_argument("-c", "--circuit", action="append", help="only this circuit (repeatable)")
    args = ap.parse_args(argv)
    bad = sum(report(nl.resolve_file(f), args.circuit) for f in args.files)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
