# Tools (logisim/tools/)

Run them from anywhere. A bare file name is looked up in `logisim/circuits/`. On this Windows machine, use Windows paths for Python.

| Script | Use |
|---|---|
| `netlist.py file [-c C] [--flat TOP] [--json]` | Read a circuit as components and nets. Don't read the XML. |
| `circgen.py` (library) | Build circuits from a Python design script, see below |
| `circcheck.py file [-c C] [--strict]` | Wiring check: ports in the middle of a wire, overlaps, dangling ends, floating inputs, wires through chips; counts crossings |
| `conventions.py file [-c C]` | HBC-2 rules (conventions.md) plus a summary of chips and ports |
| `render.py file [-c C] [--check]` | Approximate PNG of a layout, so I can review it without Logisim. PNGs are git-ignored. |
| `logitest.py file circuit --emu [module]` | Test vectors from the emulator (`hbc2 vectors`) → Logisim run headless (`logisim-evolution.exe -w`) |
| `logitest.py file circuit vectors.txt` | Run an existing vector file |

## Design scripts (logisim/designs/)
One script per generated file, e.g. `add8_ttl.py` → `circuits/add8_ttl.circ`.
- `python designs/x.py` writes the file, then runs circcheck and conventions.
- `--test` also runs the Logisim test.
- `--png` also draws a render.

circgen API, in short:
- Build: `p = Project()`, then `c = p.circuit("name")`.
- Chips: `u = c.ttl("74283", (x, y), label="U1")`. `u["A1"]` or `u[5]` gives a pin's location.
- Wires: `c.wire(p1, p2, ...)`.
- Parts: `c.input` / `c.output(loc, label, width)`, `c.tunnel`, `c.text`, `c.splitter(loc, width, ends, facing, appear, spacing)`.
- Bus helpers: `c.tap(at, to, bits, width)` and `c.bus(start, taps, width)`.
- Routing: `nested_rails(pins, toward)` gives each pin a rail depth, so wires leaving one chip edge never cross.
- Wires are split automatically at T-junctions.

## Emulator vectors (hbc2 vectors)
- `hbc2/target/release/hbc2 vectors <module> [-o file]` (or `--list`). It covers every input combination, up to 2^20.
- Modules are defined in `hbc2/crates/hbc2-cli/src/vectors.rs`: pin names and widths, plus an `eval` that calls `hbc2_core::alu`. Current modules: `add_sub8`, `add8`.
- `logitest.check_emulator(file, circuit, module=None)` rebuilds the CLI (`cargo build --release -p hbc2-cli`), writes the vectors, checks the pin names and widths, and runs them.
- `logitest.check(..., model=...)` with a Python model still exists, for quick experiments only. It is not the reference.

## Logisim CLI
- `C:\Program Files\logisim-evolution\logisim-evolution.exe -w <circuit> <vectors> <file.circ>`.
- There is no `java.exe` on this machine, so call the .exe.
- Vector format:
  - Header `name[width] ...`.
  - Values in binary; `x` means don't care.
- Failure headers go to stderr, details to stdout, with CRLF line endings.
- Timing: 131,072 vectors take about 15 s, and 524,288 take about 90 s.

## Splitter geometry (netlist.make_splitter)
- Attributes: `facing`, `fanout`, `incoming`, `appear` (left/right/center), `spacing` (× 10 px), and `bit<i>` = end index or `none`.
- With `appear=left` facing east, the ends sit in a column at x+20, above the combined port.
