# HBC-2 circuit conventions (Logisim-evolution 5.0)

These rules come from the user and apply to every circuit I create or modify. If a rule gets in the way in a specific case, **ask the user** instead of quietly breaking it, and record the decision in open-points.md or ttl-chips.md.

`tools/conventions.py` checks most of these rules automatically. `circgen`'s `save()` runs it.

## Components
- Anything that will become a chip must come from `#TTL`. The board uses 74HCT, so a `7486` in Logisim is a 74HCT86.
- Allowed for wiring: Pin, Splitter, Tunnel, Probe, Clock, Ground, Power, Constant, Text.
- RAM and EEPROM come from `#Memory` (RAM, ROM) and use the real chip's pinout, e.g. 62256 / 28C256 with `ce_n`, `oe_n`, `we_n`.
- Forbidden in a module: `#Gates`, `#Plexers`, `#Arithmetic`, and the registers and counters of `#Memory`. Test benches may use them.
- Constant is allowed in modules: it stands for inputs tied to GND/5V (e.g. the value 01 for INC).
- If a chip is missing from `#TTL`, don't build it from gates. Tell the user, and suggest a real 74HCT part in a DIP package.
- Don't wire VCC and GND. Tie every unused input to Ground (or Power if the logic needs it). Leave unused outputs unconnected.

## Circuits
- One module per circuit, named in lowercase snake_case: `add_sub8`, `alu`, `regfile`, `pc`, `sp`, `ir`, `control`, `mem_if`, `iod`.
- A test bench is named `test_<module>`. It may use non-TTL parts.
- `main` only connects modules and holds no logic.

## Files (added 2026-11-04)
- Folders are **functional units**, not modules: `logisim/circuits/<unit>/<module>.circ`, e.g. `circuits/alu/add8.circ`. Planned units: `alu/`, later `registers/`, `control/`, and so on. This keeps the number of folders down. No `modules/` folder.
- Each .circ holds one module and its `test_<module>` bench, nothing else.
- `logisim/cpu.circ` holds only `main`. It loads the modules as Logisim libraries (Project > Load Library > Logisim Library) with relative paths. A module that uses another module loads it the same way, e.g. alu.circ loads `..\add_sub8\add_sub8.circ`.
- When saving, Logisim writes library paths relative to the file, with `..\` and backslashes on Windows (seen in `LibraryManager.toRelative`). Write them the same way so a save in Logisim changes nothing.
- **Change one module at a time, with a git commit before and after each module change.**
- **If a module's ports change, tell the user which files instantiate it** (`tools/usages.py <module>`): those instances need rewiring.

## Signal names: `<module>_<signal>`, lowercase snake_case
- Pins and tunnels inside a module: `add_sub8_a`, `add_sub8_b_inv`.
- Global signals: `clk`, `rst_n`.
- Registers: `reg_a` … `reg_y`, `pc`, `sp`.
- Flags: `flag_c/e/i/n/s/z/f/h`.
- Buses: `bus_data` (8 bits), `bus_addr` (16 bits).
- Suffixes go function first, then polarity:
  - `_n` (active low) always comes last.
  - The others: `_ld`, `_oe`, `_we`, `_inc`, `_dec`, `_clr`, `_sel` (with an index: `_sel0`).
  - Use `_in` / `_out` only to resolve an ambiguity.
- The bus width goes in the Pin, never in the name. Bit 0 is the LSB. A single bit is `<bus>_<bit>`, e.g. `add_sub8_s_7`.
- Control signals are named from the receiving module's point of view: `add_sub8_sub`, not `alu_sub`.
- Keep the manual's names (`int`, `inr`, `ie`, `rw`, `re`): lowercase them and add the module prefix.

## Chips
- Label every chip `U<n>`, unique within its module, numbered top to bottom, then left to right, judged by the chip body, not the anchor. No exceptions: the user wants the rule kept simple. On the board this becomes `<module>.U<n>`.
- Add a Text next to a chip when its role isn't obvious (`bits 3..0`, `B inversion`).

## Wiring and readability
- Use one splitter per group of contiguous bits (8 → 4+4, 8 → 8), not piles of 1-bit splitters.
  - Exception approved for the 74283, see ttl-chips.md.
- Use tunnels for long or crossing connections. A tunnel's label is exactly the signal name.
- Data inputs go on the left, outputs on the right, control on top, flags at the bottom. This applies to both the schematic and the symbol.
- A custom symbol shows the module name and the real port names. Update it whenever a port changes.

## Verification (each module)
1. Run `conventions.py`: it reports forbidden parts, names and labels, file layout, and floating inputs. It follows library files.
   - A floating chip input is an error.
   - A floating input on a subcircuit instance is only a warning, since such circuits are work in progress (e.g. `alu`).
   - Also run `circcheck.py` for wiring.
2. Test exhaustively up to about 2^20 combinations. Otherwise test edge cases plus a large random sample.
   - The reference is the **emulator's own code**, not a re-implementation. `hbc2 vectors <module>` computes the expected outputs with `hbc2-core/src/alu.rs`, the same functions the CPU calls.
   - Run it with `logitest.py <file> <circuit> --emu`. A new module needs an entry in `hbc2-cli/src/vectors.rs`. If the behaviour lives inline in cpu.rs, first move it into a pure function in hbc2-core that the CPU calls.
3. Summarise in a few lines: chips per part number, ports, test result.
