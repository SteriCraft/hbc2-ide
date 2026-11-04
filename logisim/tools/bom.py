#!/usr/bin/env python3
"""Bill of materials of a circuit, from the .circ files.

Counts every chip of a circuit (#TTL parts and instances of the chip models
in circuits/chips/) and of the subcircuits it instantiates
(each instance counts), then takes the orderable references from the
"Parts in use" table of logisim/notes/chips.md. A chip missing from that
table is an error, so the circuits can only use parts that were checked.

CSV columns: part, function, qty, dip_ref, soic_ref, modules, spare
  spare    chips to buy on top of qty: 15 %, rounded up, at least 1
  modules  the circuits that hold the chip directly
The last line is the total.

Usage:
    python bom.py alu                         # print the CSV
    python bom.py alu -o ../notes/bom-alu.csv # write it
    python bom.py alu --check ../notes/bom-alu.csv   # exit 1 if the file is out of date
"""

from __future__ import annotations

import argparse
import csv
import io
import math
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist as nl  # noqa: E402

CHIPS_MD = Path(__file__).resolve().parent.parent / "notes" / "chips.md"
SPARE = 0.15
COLUMNS = ["part", "function", "qty", "dip_ref", "soic_ref", "modules", "spare"]


def parts_table(path: Path = CHIPS_MD) -> dict[str, dict]:
    """Logisim name -> row of the "Parts in use" table of chips.md."""
    text = path.read_text(encoding="utf8")
    m = re.search(r"^## Parts in use\n(.*?)(?=^## )", text, re.S | re.M)
    if not m:
        raise SystemExit(f"{path}: no '## Parts in use' section")
    rows = [line.strip().strip("|").split("|") for line in m.group(1).splitlines() if line.startswith("|")]
    head = [h.strip() for h in rows[0]]
    parts = {}
    for r in rows[2:]:
        row = dict(zip(head, (c.strip() for c in r)))
        parts[row["Logisim"]] = row
    return parts


def count(circ: nl.Circuit, chips: Counter, where: dict, n: int = 1):
    """Add the chips of circ (times n instances) and of its subcircuits."""
    for c in circ.comps:
        model = nl.chip_model(c.sub) if c.kind == "sub" and c.sub is not None else None
        if c.kind == "ttl" or model:
            chips[c.name] += n
            where[c.name].add(circ.name)
        elif c.kind == "sub" and c.sub is not None:
            count(c.sub, chips, where, n)


def bom(target: str, circuit: str | None = None) -> tuple[str, list[str]]:
    """(CSV text, errors) for a circuit, by default the one named like the file."""
    path = nl.resolve_file(target)
    proj = nl.Project(path)
    name = circuit or path.stem
    if name not in proj.circuits:
        raise SystemExit(f"{path}: no circuit {name!r}")
    chips, where = Counter(), defaultdict(set)
    count(proj.circuits[name], chips, where)
    parts = parts_table()

    errors = [f"{n}: used in {', '.join(sorted(where[n]))}, missing from chips.md 'Parts in use'"
              for n in sorted(chips) if n not in parts]
    out = io.StringIO()
    w = csv.writer(out, lineterminator="\n")
    w.writerow(COLUMNS)
    tq = ts = 0
    for n in sorted(chips, key=lambda n: int(re.sub(r"\D", "", n))):
        p = parts.get(n, {})
        qty = chips[n]
        spare = max(1, math.ceil(qty * SPARE - 1e-9))
        tq, ts = tq + qty, ts + spare
        w.writerow([p.get("Part", f"74HCT{n[2:]}?"), p.get("Function", ""), qty,
                    p.get("DIP", ""), p.get("SOIC", ""), " ".join(sorted(where[n])), spare])
    w.writerow(["total", f"{len(chips)} part numbers", tq, "", "", "", ts])
    return out.getvalue(), errors


def main(argv=None):
    ap = argparse.ArgumentParser(description="Bill of materials of a Logisim circuit.")
    ap.add_argument("file", help="circuit file, e.g. alu or circuits/alu/alu.circ")
    ap.add_argument("-c", "--circuit", help="circuit in the file (default: the file's name)")
    ap.add_argument("-o", "--out", help="write the CSV here")
    ap.add_argument("--check", metavar="CSV", help="fail if this CSV differs from the circuits")
    args = ap.parse_args(argv)
    text, errors = bom(args.file, args.circuit)
    for e in errors:
        print("error  " + e, file=sys.stderr)
    if args.check:
        same = Path(args.check).is_file() and Path(args.check).read_text(encoding="utf8").splitlines() == text.splitlines()
        print(f"{args.check}: {'up to date' if same else 'OUT OF DATE (run bom.py -o)'}")
        sys.exit(1 if errors or not same else 0)
    if args.out:
        Path(args.out).write_text(text, encoding="utf8", newline="\n")
        print(f"wrote {args.out}")
    else:
        sys.stdout.write(text)
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
