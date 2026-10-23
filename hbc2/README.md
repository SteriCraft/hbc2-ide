# HBC-2 (Rust rewrite)

New implementation of the HBC-2 emulator and IDE. The reference for its
behaviour is the HBC-2 Instruction Manual and the peripheral manuals in
`../Documentation/`. The old Qt IDE in `../old_code_archive/` stays as a reference
until this one does everything it did.

## Crates

| Crate | Role |
|---|---|
| `hbc2-core` | CPU, RAM, IOD, peripherals (monitor, keyboard, RTC, EEPROM) and `Machine`. No GUI, no threads, no dependencies. |
| `hbc2-asm` | Assembler and disassembler for `.has` files. The language is described in `../Documentation/HBC-2 Assembly Language.md`. |
| `hbc2-runner` | Runs a machine on its own thread at a chosen clock speed (1 Hz to "as fast as possible"). Commands in, state snapshots out. |
| `hbc2-cli` | `hbc2` command: assemble, headless run, speed benchmark. |
| `hbc2-ide` | `hbc2-ide`: the IDE (egui). Editor, assembler, emulator controls, and views of the CPU, memory, disassembly, IOD and peripherals. |

`templates/` holds the files of a new project (`main.has`, `interruptHandler.has`).

## Commands

```sh
cargo test                                               # run all tests
cargo run --release --bin hbc2-ide                       # the IDE
cargo run --release --bin hbc2-ide -- my/project --run   # open a project folder, assemble and run it
cargo run --release --bin hbc2 -- asm templates/main.has --symbols
cargo run --release --bin hbc2 -- run templates/main.has --ticks 10000
cargo run --release --bin hbc2 -- bench                  # emulation speed in MHz
cargo run --release --bin hbc2 -- bench --hz 100000000   # paced at 100 MHz on the emulator thread
```

## The IDE

- A project is a folder with a `main.has`, where assembly starts. File ▸ New
  project creates one from the templates.
- **F7** assembles and loads the program (paused at power-on). **F5** runs,
  **F6** pauses, **F10** runs one instruction, **Ctrl+F5** power-cycles. Run
  assembles by itself when nothing is loaded yet, or when the sources changed
  and the program has not started. A failed assembly never runs.
- Assembling uses the editors' text, saved or not.
- Click the editor margin to set a breakpoint. When the emulator pauses, the
  editor shows the line of the next instruction.
- The CPU and Memory views highlight what changed since the last run or
  step. RAM bytes can be edited in the Memory view while paused.
- Click the monitor to type on the HBC-2 keyboard (an AZERTY keyboard: see
  Help ▸ Keyboard map).
- Clock speed goes from 1 Hz to 100 MHz, or as fast as the host can go.
- Target RAM (64 KiB image) or EEPROM (1 MiB image, which plugs the EEPROM in).
  Images can be exported, and loaded back without the sources.
- Tabs can be moved, split and closed (View brings them back). The layout,
  the last project, and each project's target, peripherals, breakpoints and
  open files are remembered.

## Design notes

- One `Machine::tick()` is one clock cycle: it runs one whole instruction, or
  one step of the interrupt handshake, or nothing when halted.
- Peripherals are event-driven: a command runs once, when the CPU writes the
  peripheral's CMD port. Nothing runs on every cycle except the CPU and IOD.
- The machine is a plain value owned by whoever drives it. `hbc2-runner` moves
  it onto one emulator thread: other threads send commands through a channel
  and read the latest snapshot (a copy of the state, published about 60 times
  per second). Nothing else is shared, so neither side can block the other.
- A halted CPU with no pending interrupt is idle: `Machine::run` counts the
  remaining cycles without simulating them, and the runner sleeps.
- The IDE reads one snapshot per frame and only repaints when a new one is
  published (or on input), so a paused emulator costs nothing. Views and menus
  queue actions (`app::Action`), which run once the frame is drawn.
