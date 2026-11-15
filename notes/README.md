# HBC-2: status of the software (2026-11-15)

Everything lives in the Rust workspace `hbc2/`, plus the C compiler in `compiler/`. Run `cargo test` in `hbc2/` to check it all; the C tests need `compiler/build.cmd` first. About 330 tests pass. What is left is in [TODO.md](TODO.md).

| Part | Where | Status |
|---|---|---|
| Emulator | `hbc2/crates/hbc2-core`, `hbc2-runner` | **Done.** The reference for the hardware |
| Assembler | `hbc2/crates/hbc2-asm` | **Done** |
| C compiler | `compiler/` + `hbc2/crates/hbc2-cc` | **Finished 2026-11-11.** New work comes only from using it |
| IDE | `hbc2/crates/hbc2-ide` | **Usable day to day.** Small wishes in TODO.md |
| Command line | `hbc2/crates/hbc2-cli` (`hbc2`) | **Done.** Also generates the Logisim vectors and the microcode |
| Microcode | `hbc2/crates/hbc2-ucode` | **Done.** The 7 AT28C256 images of the control unit |

## Emulator

- **CPU:**
  - 54 instructions;
  - variable-length encoding (1–4 bytes);
  - 16-bit stack pointer, starting at 0xFE00 and growing down;
  - interrupt vector table at 0xFE00–0xFFFF;
  - invalid encodings behave as on the hardware.
  The reference is the Instruction Manual (`Documentation/`).
- **Timing equals the hardware:** every instruction takes the control unit's real cycle count (`hbc2-core/src/isa.rs`, checked against the microcode). Power-on runs the 65,548-cycle boot copy from the boot ROMs.
- **I/O:** the I/O mailbox card is modelled chip by chip (`mailbox.rs`). Behind it is a model of the ESP32 I/O processor (`iop.rs`), which runs:
  - the devices: VGA screen (native display, 256 × 192, text cells and pixels), keyboard, clock, EEPROM;
  - the 256 port latches;
  - the event queue;
  - INFO, the device list.
  The old direct IOD is kept only for the CPU's own tests (`mailbox: None`).
- **Speed:** about 240 MHz emulated with the mailbox, and 390 MHz without it; the hardware runs at 2 MHz. The runner thread paces the machine from 1 Hz to flat out.
- **Not done:**
  - the ESP32's real response times (today they are guesses);
  - the native commands BLIT, sprites, SD_READ / SD_WRITE.

## Assembler

- Implements the language of `Documentation/HBC-2 Assembly Language.md`:
  - labels, `[x:y + n]` and `[sp + n]` addressing;
  - defines, `name.msb` / `.lsb`;
  - include-once;
  - `.data`, `.bss`, `.keep`, `.library`.
- A **linker step** drops unreachable code in C builds (`compiler/LINKER.md`).
- **Outputs:**
  - a 64 KiB RAM image;
  - with `--boot-rom`, the two 32 KiB files for the boot AT28C256s (`_lo` / `_hi`);
  - symbols and a map.
- Disassembler included.
- The EEPROM target was removed on 2026-11-11: programs start from the boot ROM, and the EEPROM is storage only.

## C compiler

- Built on the **Fuzix Compiler Kit**: cc0 → cc1 → `cc2` (our HBC-2 backend) → copt → hbc2-asm. Built with MSVC (`compiler/build.cmd`).
- **Language:** C89, plus C99 declarations anywhere and in `for`, block scope, structs passed and returned by value, and `float` (IEEE binary32; `double` is the same type).
- **Not supported:** bitfields, K&R function definitions.
- **Library, all in assembly** (`compiler/lib/`):
  - stdio (printf / scanf with `%f`), string, stdlib (malloc, qsort…), math (float), time, ctype, errno;
  - `hbc2.h`: devices, screen, mailbox, EEPROM.
- **Tests:** 70 programs plus FCC's own 109, all passing. An empty program is 604 bytes; hello world is about 1.1 KB.
- **Details:** `compiler/ROADMAP.md` (decisions and steps), `compiler/ABI.md` (calling convention, runtime), `compiler/lib/README.md` (library).

## IDE

- **Projects:**
  - an assembly project (`main.has`) or a C project (`main.c`), made from templates;
  - an assembly project gets `mailbox.has` and `devices.has`.
- **Build and run:**
  - build, run, pause and step (over / into / out, in assembly or by C line), breakpoints on asm or C lines;
  - **Reset** (the board's button) and **Restart** (power cycle);
  - adjustable clock speed.
- **Views:**
  - CPU, memory (with editing), disassembly;
  - I/O: the mailbox card, the I/O processor and its log;
  - the monitor, with the keyboard (AZERTY).
- **Images:** export the RAM image or the boot ROM pair; load an image.
- **Editor:** folding, configurable shortcuts, a movable tab layout, remembered per project.
- **Installer:** `installer.cmd` builds a Windows setup with Inno Setup.
- **User guide:** `hbc2/README.md`.

## Where the notes are

| Folder | What |
|---|---|
| `notes/` (here) | Software status, and the to-do for the software |
| `logisim/notes/` | The CPU in Logisim: status, how to test and compare traces, design records, breadboard orders and test rig ([README.md](../logisim/notes/README.md), [TODO.md](../logisim/notes/TODO.md)) |
| `kicad/notes/` | Schematics and PCBs: status, parts, slot pinout, to-do ([README.md](../kicad/notes/README.md), [TODO.md](../kicad/notes/TODO.md)) |
| `Documentation/` | The manuals: the user-facing reference (CPU, assembly language, I/O, peripherals) |
| `hbc2/README.md` | Building and using the IDE, emulator and `hbc2` command |
| `compiler/` | `ROADMAP.md`, `ABI.md`, `LINKER.md`, `lib/README.md`; `compiler/notes/` holds Claude's working notes |
