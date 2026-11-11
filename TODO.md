# To-do

Everything planned or wished for, in one place. Done items are removed (git
history keeps them). Detailed notes are linked; this file only lists.

## Documentation

- **`docs/`** (the GitHub Pages site) is the Doxygen output of the old Qt
  IDE (2023); README.md no longer links to it, but still takes its logo from
  `docs/documentation/html/logo.png`. Remove it (move the logo first) or
  replace it with a user guide for the Rust IDE.
- **Instruction Manual** (`Documentation/src/HBC-2_IDE_Instruction_Manual.odt`,
  Gianni edits it): the IOD chapter becomes the mailbox card (ports, STATUS,
  TX / RX / EVT, the vector step with A8–A15 pulled down) and the ESP32
  behind it. Also check it for the removed EEPROM target. List of changes:
  `logisim/notes/iod-decisions.md`, "Docs to change".

## IDE and emulator (`hbc2/`)

- **Start-up on the hardware:** crt0 waits a fixed 32 turns for the plug-in
  interrupts; on the board the ESP32 starts after the CPU. Gianni is looking
  into it. Proposal: crt0 sends INFO and waits until every device in INFO's
  bitmap has its port (iod-decisions.md, "Open after C5").
- **Native ESP32 commands** (PUTS, PIXEL, GET_TIME, SD…; proposal 9.3), for
  screen speed: added to the I/O processor model together with the firmware
  (hardware, C4), so both have the same set.
- **I/O processor timing** (8 cycles per action, 2 for the 1 µs rule) are
  guesses: measure them on the firmware.
- **`rst_n` without a power cycle:** the machine only has power-on, which
  resets the mailbox card and the I/O processor together.
- **Later: load programs over USB.** A small loader in the boot ROM asks the
  ESP32 for the program; the IDE sends it over USB. Saves burning the two
  AT28C256 for every test. Not designed yet.

## Shell (Gianni's C project `shell_c`, not in the repo)

- **Startup message:** the computer's name, shell version, memory (RAM,
  EEPROM if present), detected devices.
- **Up arrow:** recall the last command.
- **Load / save** programs or data with the EEPROM (`memdump` / `peek` /
  `poke -e` already reach it).

## Library (`compiler/lib/asm`)

- **Fixed point** 8.8 and 16.16, with a sine table (maybe a `fixed.h` for C).

## C compiler (`compiler/`, tracking file `compiler/ROADMAP.md`)

- **Language still missing** (ABI.md 12): structs passed / returned by value,
  bitfields, block scope, `for (int i…)`.
- **Backend:** carry what the registers hold across loop labels (a dataflow
  version of `reloads.rs`; in bench.c's loops the paths into a loop label
  rarely agree: small gain expected).
- **Backend:** `x *= k` / `x /= k` in place, like `<<=`.
- **Build the C tools from cargo** (`cc` crate, one tiny bin crate per tool),
  so `cargo build` does everything without `compiler/build.cmd`.
- **Run FCC's own test programs** (upstream `test/`, not copied yet).

## Hardware

### Breadboard and test rig (`logisim/notes/breadboard-order.md`, `test-rig.md`)

- **Replay:** the Mega firmware and `tools/replay.py`; the vector exports for
  clkgen and the mem ROM pattern.
- **EEPROM programmer** firmware (built with the mem card, order 6).
- **The I/O card** isn't in the orders yet ("still a stub"): it is designed
  now (C-min, 12 chips, `bom-iod.csv`). Add its order, the 10 kΩ pull-down
  network, the ESP32 board (TTGO VGA32) and a rig map `logisim/rig/iod.map`.
- **Checks to make on the breadboard** (breadboard-checks.md B5): the IN
  drivers' turn-off overlap, A8–A15 decaying through the pull-downs in the
  vector step (fallback: put the 74HCT541 back), the 165's data hold after
  SH/LD, MBX_SEL rising while SCK is low.
- **IOD reset at CPU level** (iod-decisions.md, "Open").

### ESP32 (step C4 of io-redesign-proposal.md 9.9, once hardware is built)

- **Firmware:** the compat commands, NOP, INFO, the plug-in events, then the
  native commands. On the Mega rig first, then the TTGO VGA32. **Check
  GPIO 36 on the board's schematic before wiring.**

### KiCad (`kicad/`, Gianni)

- **Schematics still to draw:** addr, cu, iod, mem (folders `*_TODO`).
  Check each one against Logisim with `logisim/tools/kicadcmp.py`.
- **PCBs** (DIP, sockets for every chip): once the card connector pinout is
  decided.
