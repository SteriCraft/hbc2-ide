# Logisim and breadboards: to do

The circuits themselves are finished. What is left is testing them on real chips.

## Open questions

- None.

## Breadboard tests (test-rig.md, breadboard-order.md)

- **Replay:** write the Mega firmware and `tools/replay.py`. Each row of a vector file goes like this: shift in the drive bits, set the clocks, then sense with PULL high and with PULL low.
- **Vector exports** still missing:
  - clkgen: its sequences are Python in `designs/clkgen.py`, not `hbc2 vectors`;
  - the mem ROM pattern: `vectors/mem.rs` `rom_pattern`, to burn into the two AT28C256.
- **EEPROM programmer firmware:** the Mega with a ZIF socket, built with the mem card (order 6).
- **iod on the breadboard:**
  - add its order to breadboard-order.md: the 12 chips, the 33 Ω and 220 Ω resistors, the 1N5817, the TTGO cables, the TTGO VGA32 v1.4;
  - add a rig map `rig/iod.map`. Without the TTGO, U12 needs 3.3 V on J2.8, which the Mega's 3.3 V pin can supply.
- **Scope checks** ([breadboard-checks.md](breadboard-checks.md)):
  - B1: control register → 377 hold times;
  - B3: the reset;
  - B4: ROM tDF overlap at the boot's 7FFF → 8000 and FFFF → 0000;
  - B5: IN driver turn-off overlap, A8–A15 decaying through the pull-downs, the 165's data hold, MBX_SEL rising while SCK is low;
  - B6: the FabGL checks;
  - B7: the TTGO cables (slow MBX_SEL / RCLK_EVT edges, SCK ringing, flashing over USB while the HBC-2 runs).

## ESP32 firmware (iod-decisions.md, Documentation/HBC-2_Input_Output.md)

- The compat commands, NOP, INFO with its descriptors, dropping a half-received command on `rst_n`, the events, then the native commands (on FabGL). Develop on the Mega rig or a plain ESP32 first, then on the TTGO VGA32 v1.4 (not bought yet).
- Measure the firmware's real response times, and put them in the emulator's I/O processor model (`IopConfig`: today 8 cycles per action and 2 for the 1 µs rule, both guesses).

## For information (no action planned)

- At 85 °C the iod's IN drivers turn off in up to 121 ns. Breadboard check B5 (a) measures it.
- The 165's data hold after SH/LD isn't specified by TI. The estimated margin is about 90 ns.
- `kicadcmp.py`: merging Logisim's `X_in` / `X_out` nets would make addr and iod report "equivalent". This is listed in kicad/notes/TODO.md.
