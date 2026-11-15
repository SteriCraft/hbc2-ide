# Software: to do

The IDE, emulator, assembler and C compiler. Done items are removed (git
history keeps them). Status: [README.md](README.md).

## Documentation

- **Website:** a site for the project (user guide of the IDE, the manuals),
  later. The old Qt Doxygen site (`docs/`) is gone; the logo is now
  `Documentation/images/logo.png`.

## IDE and emulator (`hbc2/`)

- **Native ESP32 commands:** text, graphics, VSYNC, GET_TIME done in the
  emulator and the library (2026-11-11, Peripherals/Native/Reference.md).
  Left: BLIT, sprites, SD_READ / SD_WRITE, and the firmware itself (C4).
- **BLIT (later):** the rectangle form, decided 2026-11-14: `BLIT x y w h`
  then the pixels (4-bit, 2 per byte). At most 255 bytes of pixels per
  command, like PUTS, so interrupts stay off no longer than today; the
  library cuts a bigger image into bands of rows. Spec in
  Peripherals/Native/Reference.md (opcode `0x40`), emulator, library.
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

Listed in their own folders:
- [logisim/notes/TODO.md](../logisim/notes/TODO.md): breadboard tests, rig replay, EEPROM programmer, ESP32 firmware.
- [kicad/notes/TODO.md](../kicad/notes/TODO.md): what is left before the PCBs, and the PCBs.
