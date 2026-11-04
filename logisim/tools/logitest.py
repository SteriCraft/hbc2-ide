#!/usr/bin/env python3
"""Test a Logisim-evolution circuit against a Python model.

Writes a Logisim test vector from a Python function, runs Logisim headless
(--test-vector) and reports the result.

From Python:

    import logitest
    ok = logitest.check(
        "add8_ttl.circ", "add8",
        inputs={"add8_a": 8, "add8_b": 8, "add8_cin": 1},
        outputs={"add8_s": 8, "add8_cout": 1},
        model=lambda add8_a, add8_b, add8_cin: ...,
    )

Inputs are tested exhaustively when there are at most 2**exhaustive_bits
combinations (default 2**20). Otherwise: every combination of edge values
(0, 1, max, msb, msb-1, alternating bits) when there are few enough, then
random rows. The model returns a dict (or a tuple in output order); a value
of None means "don't care".

Against the emulator (the reference for CPU modules): the vectors come from
`hbc2 vectors <module>`, which computes the expected outputs with the
emulator's own ALU code (hbc2-core/src/alu.rs). The hbc2 CLI is rebuilt with
cargo first when needed.

    ok = logitest.check_emulator("cpu.circ", "add_sub8")

From the command line:

    python logitest.py cpu.circ add_sub8 --emu          # emulator vectors
    python logitest.py add8_ttl.circ add8 vectors.txt   # an existing vector file

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


def rows_for(inputs: dict, n_random: int = 500, exhaustive_bits: int = 20, edge_limit: int = 2000,
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


HBC2_DIR = Path(__file__).resolve().parents[2] / "hbc2"


def emulator_vectors(module: str, out: Path) -> None:
    """Write the emulator's vectors for `module` (hbc2 vectors) to `out`."""
    build = subprocess.run(["cargo", "build", "--release", "-q", "-p", "hbc2-cli"], cwd=HBC2_DIR,
                           capture_output=True, text=True, errors="replace")
    if build.returncode:
        raise SystemExit("error: cargo build failed:\n" + build.stderr)
    exe = HBC2_DIR / "target" / "release" / ("hbc2.exe" if os.name == "nt" else "hbc2")
    run = subprocess.run([str(exe), "vectors", module, "-o", str(out)], capture_output=True, text=True,
                         errors="replace")
    if run.returncode:
        raise SystemExit(f"error: hbc2 vectors {module}: {run.stderr.strip()}")


def check_emulator(circ_file, circuit: str, module: str | None = None, keep: str | None = None) -> bool:
    """Test `circuit` with the vectors of emulator module `module` (default:
    same name). Returns True when all pass."""
    with tempfile.TemporaryDirectory() as tmp:
        vec = Path(keep) if keep else Path(tmp) / f"{circuit}.vec"
        emulator_vectors(module or circuit, vec)
        with open(vec, encoding="utf-8") as f:
            head = f.readline().split()
        pins = _pins(circ_file, circuit)
        for cell in head:
            name, _, w = cell.partition("[")
            width = int(w.rstrip("]")) if w else 1
            if name not in pins:
                raise SystemExit(f"error: {circuit} has no pin {name}; pins: {', '.join(sorted(pins))}")
            if pins[name] != width:
                raise SystemExit(f"error: pin {name} is {pins[name]} bits wide, not {width}")
        n = sum(1 for _ in open(vec, encoding="utf-8")) - 1
        print(f"{circuit}: running {n} vectors from the emulator (hbc2 vectors {module or circuit})")
        return run_vectors(circ_file, circuit, vec, timeout=1800)


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
    ap.add_argument("vectors", nargs="?", help="vector file (or use --emu)")
    ap.add_argument("--emu", nargs="?", const="", metavar="MODULE",
                    help="vectors from the emulator (hbc2 vectors MODULE, default: the circuit name)")
    args = ap.parse_args(argv)
    path = nl.resolve_file(args.file)
    if args.emu is not None:
        ok = check_emulator(path, args.circuit, args.emu or None)
    elif args.vectors:
        ok = run_vectors(path, args.circuit, args.vectors)
    else:
        ap.error("give a vector file or --emu")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
