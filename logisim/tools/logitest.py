#!/usr/bin/env python3
"""Test a Logisim-evolution circuit against a Python model.

Writes a Logisim test vector from a Python function, runs Logisim headless
(--test-vector) and reports the result.

From Python:

    import logitest
    ok = logitest.check(
        "add8_ttl.circ", "add8",
        inputs={"A": 8, "B": 8, "Cin": 1},
        outputs={"S": 8, "Cout": 1},
        model=lambda A, B, Cin: {"S": (A + B + Cin) & 0xFF, "Cout": (A + B + Cin) >> 8},
    )

Inputs are tested exhaustively when there are at most 2**exhaustive_bits
combinations (default 2**12). Otherwise: every combination of edge values
(0, 1, max, msb, msb-1, alternating bits) when there are few enough, then
random rows. The model returns a dict (or a tuple in output order); a value
of None means "don't care".

From the command line, run an existing vector file:

    python logitest.py add8_ttl.circ add8 vectors.txt

Logisim is found through $LOGISIM, then the default Windows install path,
then logisim-evolution on PATH. Combinational circuits only.
"""

from __future__ import annotations

import itertools
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist as nl  # noqa: E402

DEFAULT_EXE = r"C:\Program Files\logisim-evolution\logisim-evolution.exe"


def logisim_exe() -> str:
    for cand in (os.environ.get("LOGISIM"), DEFAULT_EXE, shutil.which("logisim-evolution")):
        if cand and Path(cand).exists():
            return cand
    raise SystemExit("error: Logisim-evolution not found; set LOGISIM to its executable")


def edge_values(width: int) -> list[int]:
    m = (1 << width) - 1
    vals = {0, 1, m, 1 << (width - 1), (1 << (width - 1)) - 1,
            int("01" * width, 2) & m, int("10" * width, 2) & m}
    return sorted(vals)


def rows_for(inputs: dict, n_random: int = 500, exhaustive_bits: int = 12, edge_limit: int = 2000,
             seed: int = 1) -> list[tuple]:
    widths = list(inputs.values())
    if sum(widths) <= exhaustive_bits:
        return list(itertools.product(*(range(1 << w) for w in widths)))
    rows = []
    combos = 1
    for w in widths:
        combos *= len(edge_values(w))
    if combos <= edge_limit:
        rows += list(itertools.product(*(edge_values(w) for w in widths)))
    rng = random.Random(seed)
    rows += [tuple(rng.randrange(1 << w) for w in widths) for _ in range(n_random)]
    return rows


def _fmt(v, width):
    if v is None:
        return "x" * width
    return format(v & ((1 << width) - 1), f"0{width}b")


def write_vectors(path: Path, inputs: dict, outputs: dict, model, rows) -> int:
    names_in, names_out = list(inputs), list(outputs)
    head = [f"{n}[{w}]" if w > 1 else n for n, w in list(inputs.items()) + list(outputs.items())]
    with open(path, "w", encoding="utf-8") as f:
        f.write(" ".join(head) + "\n")
        for row in rows:
            res = model(**dict(zip(names_in, row)))
            if not isinstance(res, dict):
                res = dict(zip(names_out, res if isinstance(res, (tuple, list)) else (res,)))
            cells = [_fmt(v, inputs[n]) for n, v in zip(names_in, row)]
            cells += [_fmt(res.get(n), outputs[n]) for n in names_out]
            f.write(" ".join(cells) + "\n")
    return len(rows)


def run_vectors(circ_file, circuit: str, vec_file, show: int = 10, timeout: int = 600) -> bool:
    """Run a vector file in Logisim. Prints the summary and the first failures."""
    circ_file, vec_file = Path(circ_file).resolve(), Path(vec_file).resolve()
    proc = subprocess.run([logisim_exe(), "-w", circuit, str(vec_file), str(circ_file)],
                          capture_output=True, text=True, errors="replace", timeout=timeout,
                          cwd=tempfile.gettempdir())
    out = (proc.stdout + proc.stderr).replace("\r\n", "\n").replace("\r", "\n")
    m = re.search(r"Passed:\s*(\d+),\s*Failed:\s*(\d+)", out)
    if not m:
        print(out.strip()[-2000:])
        print("error: no test result from Logisim (wrong circuit or pin names?)")
        return False
    passed, failed = int(m.group(1)), int(m.group(2))
    print(f"{circuit}: {passed} passed, {failed} failed")
    if failed:
        # headers ("Error on test vector N:") go to stderr, details to stdout
        heads = re.findall(r"Error on test vector (\d+):", proc.stderr)
        details = re.findall(r"(?:^  .*\n?)+", proc.stdout.replace("\r\n", "\n"), re.M)
        errs = [f"vector {h}: " + d.strip().replace("\n  ", "; ") for h, d in zip(heads, details)]
        for e in errs[:show]:
            print("  " + e.strip().replace("\n", "\n  "))
        if len(errs) > show:
            print(f"  ... {len(errs) - show} more")
    return failed == 0


def check(circ_file, circuit: str, inputs: dict, outputs: dict, model, rows=None,
          keep: str | None = None, **opts) -> bool:
    """Generate vectors from `model`, run them, return True when all pass.
    `rows` overrides the generated input rows; `keep` saves the vector file."""
    pins = _pins(circ_file, circuit)
    missing = [n for n in list(inputs) + list(outputs) if n not in pins]
    if missing:
        raise SystemExit(f"error: {circuit} has no pin {', '.join(missing)}; pins: {', '.join(sorted(pins))}")
    for n, w in list(inputs.items()) + list(outputs.items()):
        if pins[n] != w:
            raise SystemExit(f"error: pin {n} is {pins[n]} bits wide, not {w}")
    rows = rows if rows is not None else rows_for(inputs, **opts)
    with tempfile.TemporaryDirectory() as tmp:
        vec = Path(keep) if keep else Path(tmp) / f"{circuit}.vec"
        n = write_vectors(vec, inputs, outputs, model, rows)
        print(f"{circuit}: running {n} vectors")
        return run_vectors(circ_file, circuit, vec)


def _pins(circ_file, circuit) -> dict:
    proj = nl.Project(Path(circ_file))
    if circuit not in proj.circuits:
        raise SystemExit(f"error: no circuit {circuit!r} in {circ_file}")
    return {c.label: nl.parse_int(c.attrs.get("width"), 1)
            for c in proj.circuits[circuit].comps if c.kind == "pin" and c.label}


def main(argv=None):
    import argparse
    ap = argparse.ArgumentParser(description="Run a Logisim test vector file on a circuit.")
    ap.add_argument("file")
    ap.add_argument("circuit")
    ap.add_argument("vectors")
    args = ap.parse_args(argv)
    ok = run_vectors(nl.resolve_file(args.file), args.circuit, args.vectors)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
