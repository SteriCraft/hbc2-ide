# Tools (logisim/tools/)

Run them from anywhere. A bare name is looked up as `logisim/<name>`, `circuits/<name>`, then `circuits/<name>/<name>.circ` and `circuits/*/<name>.circ`, so `add_sub8`, `alu` and `cpu.circ` all work. On this Windows machine, use Windows paths for Python.

| Script | Use |
|---|---|
| `netlist.py file [-c C] [--flat TOP] [--json]` | Read a circuit as components and nets. Don't read the XML. `--flat` treats chip models (`ttl74574`) as chips, so every chip pin on a net is listed. |
| `circgen.py` (library) | Build circuits from a Python design script, including sub-circuits from other files and custom symbols; see below |
| `circcheck.py file [-c C] [--strict] [--no-libs]` | Wiring check: ports in the middle of a wire, overlaps, dangling ends, floating inputs, wires through chips; counts crossings |
| `conventions.py file [-c C] [--no-libs]` | HBC-2 rules (conventions.md), file layout included, plus a summary of chips and ports |
| `usages.py <module>` | Every instance of a module across logisim/*.circ, to report when its ports change |
| `bom.py file [-c C] [-o CSV] [--check CSV]` | Bill of materials: counts the chips of a circuit and of every subcircuit instance, and takes the DIP/SOIC references from chips.md "Parts in use". A chip missing from that table is an error. Pull Resistors give a row of bussed 8-resistor networks (one per 8 lines pulled the same way in a circuit). `designs/alu.py` writes notes/bom-alu.csv with it. |
| `timing.py [--section alu\|flags\|regs\|addr\|mem\|cu] [--limit NS] [--duty PCT]` | Path delays (typ, max 25 °C, max 85 °C) from chips.md "Delays". **Pass criterion: max 25 °C** with bus loading; each path with a limit (`LIMITS`: the 500 ns step, another path such as the earliest /WE fall, or a chip minimum) shows its margin at 25 °C (pass, exit 1 on a failure) and at 85 °C (information only). `--limit NS` adds a limit to the paths that have none. A stage `("-", part, path, what)` subtracts a chip delay (an edge that can come early), so the clock generator's times are computed per column. The paths are written by hand in the script; update them when a module changes. A stage can also be `("@", ns, what)` (a fixed time in the step, e.g. clk_wr_n) or `("path", name, what)` (another path in series, for end-to-end sums). A chip stage with a 4th element, the nets it drives (`BUS_AB`, `("bus_data",)` ...), gets the extra delay of their load beyond 50 pF: pins counted on the circuits (`netlist.flatten`), estimates for sections not built, 0.13 ns/pF for bus drivers and 0.20 ns/pF for standard outputs (`BUSES`, `SLOPE`). `--section regs` and `--section addr` also print the load table (`bus_addr` included). `--duty PCT` (default 50) moves the oscillator's falling edges (stages `("@", "fall1", …)`, `OSC`): run 45 and 55 too (the part's 45–55 %). `--section cu`: the sequencer paths (to the pipeline 574s' setup), the clock generator's, and the clock buffer branches with their loads and skew (`CLOCK_BRANCHES`). |
| `render.py file [-c C] [--check]` | Approximate PNG of a layout, so I can review it without Logisim. PNGs are git-ignored. |
| `logitest.py file circuit --emu [module]` | Test vectors from the emulator (`hbc2 vectors`) → Logisim run headless (`logisim-evolution.exe -w`). Big files run in chunks of 524,288 rows; sequential files are cut between sets only (each chunk starts at power-up), so a set must stay under that |
| `logitest.py file circuit vectors.txt` | Run an existing vector file |
| `logitest.py file --open` | Does Logisim open the file? It runs `-tty stats` headless (loads and counts parts, about 1 s; `-tty table` simulated every input and timed out on alu), and a missing library fails. |

## Microcode (hbc2 workspace)

`hbc2 ucode [-o dir] [--cycles file.md] [--check]` compiles `hbc2/crates/hbc2-ucode/microcode/hbc2.uc`, prints the rules it checked, writes the 7 AT28C256 images and `listing.txt` (`-o logisim/circuits/cu/microcode`) and the cycle table (`--cycles logisim/notes/cu-cycles.md`); `--check` runs the datapath model against the emulator. Rerun it after any change to the source and commit the outputs (cu-decisions.md section 13).

## Design scripts (logisim/designs/)
One script per generated file, e.g. `add_sub8.py` → `circuits/alu/add_sub8.circ`.
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
- Libraries: `lib = p.library(path_to_other.circ)`, then `u = c.sub(lib, "add_sub8", loc)`. `u["add_sub8_a"]` gives a port's location, computed from the other file's symbol. On save, the relative path is written the way Logisim writes it (backslashes on Windows).
- Symbol (ports always on the 10 px grid): `c.symbol(top=[...], left=[...], right=[...], bottom=[...])`, listing pin labels. It writes a custom appearance: a box sized to the labels, full port names, the module name in bold, and the anchor on the first right port. Without it, the module gets Logisim's default symbol (inputs left, outputs right). See designs/alu.py.
- Wires are split automatically at T-junctions. This can join a rail to a Ground or Power it passes by: circcheck reports a constant that shares a net with a driver.
- `rails_left(c, chip, signals, line_x, taps, ends, pitch=20)`: the add_sub8 style for any chip facing east. Each pin goes to a bus tap, a 1-bit line, Ground/Power (`"gnd"`/`"vcc"`) or a tunnel (`("tunnel", name)`, use pitch=30). Edges come from the real pin positions (the 74153 is taller).
- `c.move(dx, dy)` shifts the whole drawing, so a script can reproduce a layout Gianni shifted by hand (add_sub8: 210, 90; alu: 160, 30, before the 2026-11-04 rebuild).
- Vertical splitters (facing north/south) put end 0 at the right: use ends `[[7 - k] for k in range(8)]` to have bit 0 on the left.
- Chips facing south are drawn as the datasheet's top view (pin 1 top left). `pin_tunnels(c, chip, {pin: signal})` puts a tunnel on each pin listed (`"gnd"`/`"vcc"` give Ground/Power), on every side (`levels=6, stagger=20` for 20 px pitch symbols such as a model's DIP, designs/regs.py); `bus_tunnels(c, at, name, width, output=False, names=None)` gives a module pin with a splitter to tunnels `name_k` (`names` overrides them, e.g. `ir_r3` = V1's bits). See designs/flags.py.

## Chip models (circuits/chips/)
A chip missing from #TTL is a circuit named after the part, in `circuits/chips/<part>.circ`, with its datasheet next to it (rules: conventions.md "Components").
- `c.dip_symbol(title, order)`: DIP symbol, pins in datasheet order (`order[0]` = pin 1), pin number and name on each, VCC and GND included. The anchor is pin 1.
- `t.model(chip, loc, label="U1")`: an instance of a model of the same file, with `u["D0"]` port locations (used by the `test_<part>` bench).
- netlist.py marks a model as kind "chip" (`chip_model`), numbers its ports by DIP pin (shown `pin:name`), and knows the generic parts allowed inside a model (NOT Gate, Controlled Buffer, Register, D Flip-Flop: kind "generic").
- conventions.py checks a model (VCC/GND pins present and unwired, unique pin names, a DIP symbol, only allowed parts) and treats its instances as chips (`U<n>`, floating inputs are errors); bom.py counts them.
- The model circuit is named `ttl<part>` (`netlist.chip_model` maps it back to the part number; conventions.py checks it). Instantiate it from another file with `c.sub(p.library(".../chips/74574.circ"), "ttl74574", loc)`.
- `c.dip_symbol(title, order, extra)`: `extra` = simulation-only pins (no number) on the left edge, 10 px apart (ir-mem-decisions.md M2 / M3: the RAM's `DQk_in`, the ROM's 4-bit `IMAGE`). They must be listed in `netlist.SIM_ONLY` (conventions.py checks it); `netlist.pcb_ports(comp)` gives a chip's real pins with their numbers, without them, for a future PCB netlist export (#TTL parts: VCC and GND aren't ports).
- netlist.py GENERIC also knows #Memory RAM (separate data bus, byte enables, trigger high, logisim_evolution appearance) and ROM, and a 4-input AND Gate (size 50); widths by attribute name (`addrWidth`, `dataWidth`). Multi-line attribute values (ROM contents) are written as element text.
- Done: `74574` (designs/74574.py, 2,064 datasheet rows), `as6c1008` (3,260), `at28c256` (302,912 rows: images 0–8 read in full; `--image F.bin` puts a 64 KiB boot image in images 0–1, the microcode in 2–8 comes from circuits/cu/microcode). `test_74670.circ` (designs/test_74670.py) checks Logisim's own 74670 against its datasheet (912 rows), so it has no model; `test_74377.circ` (designs/test_74377.py) does the same for the 74377 (250 rows).

## Emulator vectors (hbc2 vectors)
- `hbc2/target/release/hbc2 vectors <module> [-o file]` (or `--list`). It covers every input combination, up to 2^20.
- Modules are defined in `hbc2/crates/hbc2-cli/src/vectors.rs`: pin names and widths, plus an `eval`. Current modules: add_sub8, logic8, shift8, flags8, cmp8, rmux8, alu; sequential: flags, flags_cpu; regs, regs_cpu (in `vectors/regs.rs`); addr, addr_cpu (in `vectors/addr.rs`); ir, ir_cpu (`vectors/ir.rs`); mem (`vectors/mem.rs`); mem_cpu (`vectors/mem_cpu.rs`, for circuits/mem/test_mem_cpu.circ: writes the ROM image to `$HBC2_ROM_OUT`, run it through designs/test_mem_cpu.py --test); cu (`vectors/cu.rs`: the microcode run by the datapath model, boots and random programs with interrupts, about 4.2 million rows, 5 minutes; `HBC2_CU_SETS=n` runs fewer sets).
- `eval` uses `hbc2_core::alu` where it exists. Otherwise `exec(op, a, b, carry)` runs the real CPU on that instruction (Reg mode) and returns the register and the flags. A flag missing from the instruction's flag list comes back None, written `x`. The emulator is never changed.
- A module whose inputs are too many for an exhaustive test gives its own `rows` (rmux8). The limit is 2^21; logitest runs big files in chunks of 524,288 rows (2^21 rows take about 10 minutes).
- `logitest.check_emulator(file, circuit, module=None)` rebuilds the CLI (`cargo build --release -p hbc2-cli`), writes the vectors, checks the pin names and widths, and runs them.
- `logitest.check(..., model=...)` with a Python model still exists, for quick experiments only. It is not the reference.
- Sequential circuits use Logisim's `<set>` / `<seq>` columns: each set starts from power-up and its rows run in order. `logitest.write_sequences(path, inputs, outputs, sets)` writes them from Python, `check_sequences(file, circuit, inputs, outputs, sets)` runs them; `logitest.FLOAT` expects high impedance; `buses=` names the bus outputs, which may not be don't care. `hbc2 vectors` writes sequences for its sequential modules (flags, flags_cpu, regs, regs_cpu); their `SeqModule::buses` must never be `Out::X` (`Out::U`: driven, value unknown since power-up; counted on stderr). logitest prints `BUS CONTENTION on <pin>` for any E Logisim reports.

## Logisim CLI
- `C:\Program Files\logisim-evolution\logisim-evolution.exe -w <circuit> <vectors> <file.circ>`.
- There is no `java.exe` on this machine, so call the .exe.
- Vector format:
  - Header `name[width] ...`.
  - Values in binary; `x` means don't care.
- Failure headers go to stderr, details to stdout, with CRLF line endings.
- Timing: 131,072 vectors take about 15 s, and 524,288 take about 90 s.

## Libraries across files
- `netlist.Project` loads every `file#` library up front, the way Logisim does, and accepts backslashes. `proj.file_libs` maps id → (desc, Project or None if missing). `proj.libraries()` returns this file plus every loaded one, each once.
- circcheck and conventions check the whole tree from the file given, so `conventions.py cpu.circ` checks everything. A missing library is an error.

## Splitter geometry (netlist.make_splitter)
- Attributes: `facing`, `fanout`, `incoming`, `appear` (left/right/center), `spacing` (× 10 px), and `bit<i>` = end index or `none`.
- With `appear=left` facing east, the ends sit in a column at x+20, above the combined port.
