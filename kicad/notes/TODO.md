# KiCad: to do

Details for each item are in the notes listed in [README.md](README.md).

## To do

### Before routing

1. **Choose the capacitors** (100 nF, 47 µF, 10 µF, 1000 µF) and check their footprints. Those footprints are provisional.
2. **Choose the other open parts:** 33 Ω, 220 Ω, the potentiometer, the rocker switch, the Nano headers, the JP1 header, the iod's pin headers and Dupont wires / PS/2 cable, and the socket type (turned-pin or open-frame). See [parts.md](parts.md).
3. **Slot footprints x4 / x8 / x16:** get the Amphenol drawing or measure a real slot. Check the peg holes, then print each footprint at 1:1 and lay the slot on it. This only blocks the motherboard.
4. **Barrel jack:** check that the CUI PJ-102AH fits the GST25E05's plug.
5. **Mechanical choices for each card:**
   - card height and width, which depend on the case's guide rails;
   - mounting holes or a bracket;
   - where the clkgen buttons, potentiometer and Nano USB go, and where the iod's TTGO connectors go;
   - the iod silkscreen: each TTGO pin's signal name next to J1, J2 and J4 (the Dupont wires aren't keyed).

### PCBs

6. For each board, set up the PCB:
   - draw the outline (on the cards, the PCIe finger profile: key notch, bevel, finger depth);
   - run *Update PCB from Schematic*;
   - set the design rules: track and clearance widths, with wide power tracks.
7. **Motherboard:** 4 layers, for example signals / GND / signals + 5 V pour / signals. Route the data bus and the addr / cu block across the ir, addr and cu slots first: they are the most crowded rows.
8. **Suggested first card:** clkgen. It is small, and it comes first in the breadboard order.
9. **Before ordering:**
    - DRC;
    - one more `kicadcmp.py` run and ERC on every board;
    - order a bare test card with one slot to check the fit.

### Later / optional

- Teach `kicadcmp.py` to merge Logisim's `X_in` / `X_out` nets, so that addr and iod report "equivalent".
- A front-panel reset button: a 2-pin header in parallel with clkgen SW3.

## In progress

- Nothing.

## Done

- 2026-11-11: electrical review of 8 boards: no wiring error; every finding closed by 2026-11-15 (the review note was removed then; it is in git history).
- 2026-11-13: cu drawn. It compares as equivalent.
- 2026-11-14:
  - iod with the TTGO VGA32: SN74LVC244AN at 3.3 V (H1), series resistors, the 1N5817 feed.
  - A8–A15 pull-downs moved to the motherboard (H2). H3 is not needed.
  - clkgen: oscillator Y1, reset RC (H5), Arduino Nano clock / step driver, test points.
  - Power (S1): series diodes and TVS removed from the cards; protection is now at the motherboard input.
  - Footprints on every board except the connectors (H4). The cu's 60 pull resistors became 8 SIP-9 networks.
  - Card pins cut to 410 for 164 CPU signals ([motherboard-netmap.md](motherboard-netmap.md)).
  - PCIe slot pinout approved ([slot-pinout.md](slot-pinout.md)); slot parts chosen (Amphenol FCI 10018783).
- 2026-11-15:
  - Motherboard: 9 slots, RN1, power input. ERC 0 / 0, nets checked against Logisim.
  - Every card's connectors replaced by one PCIe card edge, checked pin for pin against its slot.
  - References renumbered on every board (by Gianni).
  - Part labels checked against chips.md (S3).
  - ERC 0 / 0 on all 10 schematics.
  - iod: J1, J2, J4 get plain vertical pin headers for Dupont wires (Gianni); every part on every board now has a footprint. ERC 0 / 0, netlist unchanged.
  - iod D1 and clkgen D2: 1N5817 → 1N5819 (Gianni has 10 in stock; same DO-41 footprint).
