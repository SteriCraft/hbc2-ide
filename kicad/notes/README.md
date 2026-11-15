# KiCad: status of the schematics (2026-11-15)

The HBC-2 is built as **9 vertical cards in PCIe slots on a motherboard**. Every chip sits in a DIP socket, and the chips are the ones tested on the breadboards. You draw the schematics in KiCad 10. Claude checks them against the Logisim circuits and edits them by script when you ask.

**Where things stand:** every schematic is drawn and checked. The PCBs aren't started: each `.kicad_pcb` file is still empty. What is left before routing is in [TODO.md](TODO.md).

## Boards

| Board | Project | Chips | Edge / slot | Against Logisim (`kicadcmp.py`) | ERC |
|---|---|---|---|---|---|
| flags | `flags/hbc2_flags` | 15 | x4, slot J2 | equivalent | 0 / 0 |
| alu | `alu/hbc2_alu` | 29 | x8, J4 | equivalent | 0 / 0 |
| registers | `registers/hbc2_registers` | 18 | x16, J6 | equivalent (`circuits/registers/regs.circ`) | 0 / 0 |
| ir | `ir/hbc2_ir` | 7 | x16, J8 | equivalent | 0 / 0 |
| addr | `addr/hbc2_addr` | 34 | x16, J10 | 8 expected differences: Logisim splits the low address byte into in / out nets | 0 / 0 |
| cu | `cu/ctrl_unit/hbc_ctrl_unit` | 30 + 8 resistor networks | x16, J3 | equivalent (`circuits/cu/cu.circ`) | 0 / 0 |
| mem | `mem/hbc2_mem` | 8 | x8, J5 | equivalent | 0 / 0 |
| iod | `iod/hbc2_iod` | 12 | x4, J7 (edge is J3 on the card) | no wiring error; the expected differences are the TTGO side and the in / out split | 0 / 0 |
| clkgen | `cu/clkgen/hbc2_clkgen` | 5 + oscillator + Nano driver | x1, J9 | equivalent (`circuits/cu/clkgen.circ`) | 0 / 0 |
| motherboard | `mb/hbc2_mb` | — | 9 slots, A8–A15 pull-downs, power input | nets checked against Logisim ([motherboard-netmap.md](motherboard-netmap.md)) | 0 / 0 |

- **Card edges:** one card edge per card, drawn with the custom symbol `custom_symbols:HBC2_PCIe_xN` and the footprint `Connector_PCBEdge:BUS_PCIexpress_xN`. Each edge matches its motherboard slot pin for pin.
  - The same symbol is used on the motherboard, with the slot footprint `hbc2_slots:PCIe_xN_Slot_Amphenol_…`.
  - Both footprints name their pads A1…/B1…, so the shared symbol connects the same pins on both sides.
- **Footprints:** every part has one, **except the three TTGO connectors on the iod** (J1, J2, J4). The capacitor footprints are provisional until the parts are chosen.
- **Power:**
  - Cards have no diode on their supply. Power comes in on the edge: B = +5V, A = GND, on pins 1, 7, 13 …
  - The only protection is at the motherboard input: barrel jack → switch → fuse T 3.15 A, then a TVS and 1000 µF across the rail.
- **Unused pins:**
  - Every unused input is tied.
  - Every unused output has a no-connect flag.
  - Every chip has VCC, GND and its own 100 nF.

## Notes in this folder

| File | What |
|---|---|
| [TODO.md](TODO.md) | To do, in progress and done, up to the PCBs |
| [parts.md](parts.md) | Every chip and component per board, with references and what is still to choose |
| [slot-pinout.md](slot-pinout.md) | Slot order and sizes, the full A/B pinout, footprint geometry, track count across the slots (why 4 layers) |
| [motherboard-netmap.md](motherboard-netmap.md) | Every CPU signal: which card pin goes to which slot pin |
| [design-notes.md](design-notes.md) | Board-level design: card slots, clkgen oscillator / reset / Nano driver, power (S1), iod and the TTGO wiring |

## Checking a schematic

From `logisim/`:

```
python tools/kicadcmp.py circuits/<unit>/<circ>.circ ../kicad/<unit>/hbc2_<unit>.kicad_sch
```

- `kicadcmp.py` compares chip pins by type and net name, so the references don't have to match.
- Run KiCad's ERC as well: it checks no-connect flags and pin types, which the comparer doesn't.
- `kicad-cli` is at `C:\Program Files\KiCad\10.0\bin`. Netlist export:
  `kicad-cli sch export netlist -o out.net <file>.kicad_sch`.

## Libraries

- `custom_symbols.kicad_sym`: the PCIe edge / slot symbols, written by `lib/gen_pcie_symbols.py`.
- `lib/hbc2_slots.pretty`: the slot footprints, written by `lib/gen_pcie_slots.py`. They are registered in `mb/fp-lib-table`.
