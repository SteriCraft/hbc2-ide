# HBC-2 emulator and IDE (Rust)

The emulator, assembler, C compiler driver and IDE of the HBC-2. What the
machine does is defined by the documents in `../Documentation/`:

| Topic | Where |
|---|---|
| CPU and instruction set | `HBC-2_Instruction_Manual.pdf`, `HBC-2_Instruction_Set.pdf` |
| Assembly language, `.has` files, boot ROM export | `HBC-2 Assembly Language.md` |
| Devices (monitor, keyboard, RTC, EEPROM) and the I/O mailbox | `Peripherals/README.md`, `HBC-2_Input_Output.pdf` |
| C: calling convention, startup, library | `../compiler/ABI.md`, `../compiler/lib/README.md` |
| Hardware design (Logisim), I/O redesign | `../logisim/notes/` (`iod-decisions.md` for the mailbox) |

## Crates

| Crate | Role |
|---|---|
| `hbc2-core` | CPU, RAM, the I/O mailbox card, the I/O processor model (the ESP32, which runs the devices), peripherals and `Machine`. No GUI, no threads, no dependencies. |
| `hbc2-asm` | Assembler and disassembler for `.has` files. |
| `hbc2-runner` | Runs a machine on its own thread at a chosen clock speed (1 Hz to "as fast as possible"). Commands in, state snapshots out. |
| `hbc2-cc` | C compiler driver: runs the Fuzix Compiler Kit stages of `../compiler` (built by `../compiler/build.cmd`) and assembles the result with the C runtime and library. |
| `hbc2-ucode` | Microcode of the hardware control unit: source language, generator (7 AT28C256 images), datapath model checked against the emulator. |
| `hbc2-cli` | The `hbc2` command (below). |
| `hbc2-ide` | The IDE (egui): editor, build, emulator controls, views of the CPU, memory, disassembly, I/O and devices. |

`templates/` holds the files of a new assembly project (`main.has`,
`mailbox.has`, `devices.has`; `main_bare.has` without the devices) and of a
C project (`main.c`). `mailbox.has` and `devices.has` are copies of the C
library's (`../compiler/lib/asm/`); a test checks they match.

## Commands

```sh
cargo test                                               # all tests (the C ones need ../compiler/build.cmd first)
cargo run --release --bin hbc2-ide                       # the IDE
cargo run --release --bin hbc2-ide -- my/project --run   # open a project folder, build and run it
cargo run --release --bin hbc2 -- asm templates/main.has --symbols
cargo run --release --bin hbc2 -- asm main.has --boot-rom   # also main_lo.bin / main_hi.bin for the two AT28C256
cargo run --release --bin hbc2 -- run templates/main.has --ticks 10000
cargo run --release --bin hbc2 -- cc main.c other.c -o program.bin
cargo run --release --bin hbc2 -- run main.c --ticks 1000000 --eeprom eeprom.bin
cargo run --release --bin hbc2 -- bench                  # emulation speed in MHz
```

`hbc2` with no arguments lists everything, including `trace` / `compare`
(execution traces), `vectors` (Logisim test vectors) and `ucode` (the
microcode images).

## Installer (Windows)

`installer.cmd` (at the root of the repository) builds the programs and
`installer/output/HBC-2-IDE-<version>-setup.exe` with Inno Setup 6
(`winget install JRSoftware.InnoSetup`). The C compiler must be built first
(`compiler/build.cmd`). The installer puts the IDE, `hbc2.exe` and the
compiler's files (`compiler/`) in one folder, `Program Files\HBC-2 IDE` by
default (it asks for administrator rights); the IDE finds the compiler
next to its executable. The version is the workspace's (`Cargo.toml`).

## The IDE

- **Projects.** A project is a folder with a `main.has` (assembly) or a
  `main.c` (C). File ▸ New project creates one from the templates.
- **Building.** F7 assembles (using the editors' text, saved or not) and
  loads the program, paused at power-on. A C project is saved first, then
  every `.c` file is compiled and assembled with every `.has` file of the
  folder, the startup code, the runtime and the C library, in the
  background (compiled files go to the hidden `.build/`). Errors point at
  the C lines. The startup code, runtime and library open read-only.
- **Running.** F5 runs (it builds first when nothing is loaded, or when the
  sources changed and the program has not started), F6 pauses, Ctrl+F5
  power-cycles. A failed build never runs.
- **Stepping.** One instruction in assembly, one line in C. F10 steps over
  (called routines and interrupt handlers run until they return, unless a
  breakpoint stops them), F11 steps into them, Shift+F11 steps out. In C,
  F11 stops at the next line, inside a called C function, or at the first
  instruction of an assembly file. Click the editor margin for a
  breakpoint; breakpoints and the current line work on the C source.
- **Views.** The CPU and Memory views highlight what changed since the last
  run or step; RAM bytes can be edited while paused. Click the monitor to
  type on the HBC-2 keyboard (AZERTY: Help ▸ Keyboard map).
- **Devices.** Emulator ▸ Peripherals plugs or unplugs each device (it
  resets the machine). The EEPROM starts blank and keeps what programs
  write across resets and new builds; the Emulator menu saves its content
  to a file and loads it back.
- **Images.** The Assembler menu exports the 64 KiB RAM image, or the same
  image as the boot ROM's two files. Emulator ▸ Load image runs an image
  without its sources.
- **Editor.** Functions fold (arrows in the margin; Ctrl+K collapses all,
  Ctrl+J expands). Settings ▸ Shortcuts lists and changes every shortcut.
- **Layout.** Tabs can be moved, split and closed (View brings them back).
  The layout, the last project, and each project's peripherals,
  breakpoints and open files are remembered.

## Design notes

- One `Machine::tick()` is one clock cycle. An instruction takes the cycles
  of the hardware (`hbc2-core/src/isa.rs`); the machine reports an
  instruction boundary on its last cycle.
- Devices are event-driven: a command runs once, when the I/O processor
  delivers a write to its port. Nothing else runs on every cycle but the
  CPU, the mailbox card and the I/O processor model, and those two are
  skipped while they have nothing to do. Without the mailbox
  (`MachineConfig::mailbox` = None, used by the CPU tests) the CPU talks to
  the IOD directly, as the first design did.
- The machine is a plain value owned by whoever drives it. `hbc2-runner`
  moves it onto one emulator thread: other threads send commands through a
  channel and read the latest snapshot (a copy of the state, published about
  60 times per second). Nothing else is shared, so neither side can block
  the other.
- A halted CPU with no pending interrupt is idle: `Machine::run` counts the
  remaining cycles without simulating them, and the runner sleeps.
- The IDE reads one snapshot per frame and only repaints when a new one is
  published (or on input), so a paused emulator costs nothing. Views and
  menus queue actions (`app::Action`), which run once the frame is drawn.
