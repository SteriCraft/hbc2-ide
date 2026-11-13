# To-do

Everything planned or wished for, in one place. Done items are removed (git
history keeps them). Detailed notes are linked; this file only lists.

## Documentation

- **Website:** a site for the project (user guide of the IDE, the manuals),
  later. The old Qt Doxygen site (`docs/`) is gone; the logo is now
  `Documentation/images/logo.png`.

## IDE and emulator (`hbc2/`)

- **Native ESP32 commands:** text, graphics, VSYNC, GET_TIME done in the
  emulator and the library (2026-11-11, Peripherals/Native/Reference.md).
  Left: BLIT, sprites, SD_READ / SD_WRITE, and the firmware itself (C4).
- **ESP32 timing in the emulator:** the I/O processor model answers after 8
  CPU cycles per action (2 for the 1 µs rule). These are guesses: replace
  them with the firmware's measured times.
- **ESP32 boot time in the IDE:** `IopConfig::boot_cycles` is 0 by default
  (programs start at once); the board's is about 1 s. Maybe an option.
- **Later: load programs over USB.** A small loader in the boot ROM asks the
  ESP32 for the program; the IDE sends it over USB. Saves burning the two
  AT28C256 for every test. Not designed yet.

## Shell (Gianni's C project `shell_c`, not in the repo)

- **Rewrite it for the new I/O system** (the mailbox, `mbx_out` / `mbx_in`).
  It only calls the library, which already goes through the mailbox, and
  it builds unchanged (2026-11-11): what's left is checking each command
  in the IDE and dropping what no longer fits.
- **Up arrow:** recall the last command.

## C compiler (`compiler/`, tracking file `compiler/ROADMAP.md`)

- Nothing planned. Dropped by Gianni (2026-11-11): bitfields and K&R
  definitions (ABI.md 12), building the C tools from cargo, carrying
  register contents across loop labels (small gain expected); `fixed.h`
  (2026-11-13).

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

### ESP32 (step C4 of io-redesign-proposal.md 9.9, once hardware is built)

- **Firmware:** the compat commands, NOP, INFO (descriptors: class,
  product ID, first port, number of ports), dropping a half-received
  command on `rst_n`, the events, then the native commands. No plug-in
  events, no mouse, no audio. On the Mega rig first, then the TTGO VGA32.

### KiCad (`kicad/`, Gianni)

- **Schematics:** all drawn (cu committed 2026-11-13). What is left
  before the PCBs is in `kicad/NOTES.md`, "To do before the PCBs".
- **PCBs** (DIP, sockets for every chip): once the card connector pinout is
  decided.
