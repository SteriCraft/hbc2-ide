#!/usr/bin/env python3
"""List where a module is instantiated, across every .circ under logisim/.

When a module's ports change, these instances need rewiring.

Usage:
    python usages.py add_sub8          # module file found as circuits/add_sub8/add_sub8.circ
    python usages.py add8 --file circuits/alu/add8.circ
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import circcheck  # noqa: E402
import netlist as nl  # noqa: E402


def usages(module: str, module_file: Path) -> list[tuple[Path, str, tuple, str]]:
    """(file, circuit, location, label) of every instance of `module` from `module_file`."""
    target = module_file.resolve()
    found = []
    for f in nl.all_circ_files():
        proj = nl.Project(f)
        for cname, circ in proj.circuits.items():
            for e in circ.elem.findall("comp"):
                if e.get("name") != module:
                    continue
                lib = e.get("lib")
                if lib is None:
                    src = f.resolve()                       # circuit of the same file
                else:
                    desc = proj.libs.get(lib, "")
                    if not desc.startswith("file#"):
                        continue
                    src = proj.lib_path(desc)
                if src == target:
                    found.append((f, cname, nl.parse_loc(e.get("loc")), nl.read_attrs(e).get("label", "")))
    return found


def main(argv=None):
    ap = argparse.ArgumentParser(description="List the instances of a module in every logisim/ .circ file.")
    ap.add_argument("module")
    ap.add_argument("--file", help="the module's .circ (default: found from the module name)")
    args = ap.parse_args(argv)
    mfile = nl.resolve_file(args.file or args.module)
    hits = usages(args.module, mfile)
    if not hits:
        print(f"{args.module} ({circcheck._rel(mfile)}) is not instantiated anywhere")
    for f, cname, loc, label in hits:
        print(f"{circcheck._rel(f)}: circuit {cname}, at {loc}" + (f", label {label}" if label else ""))


if __name__ == "__main__":
    main()
