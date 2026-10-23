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

Planned: `hbc2-ide` (egui GUI).

`templates/` holds the default project files (`main.has`, `interruptHandler.has`).

## Commands

```sh
cargo test                                          # run all tests
cargo run --release -- asm templates/main.has --symbols
cargo run --release -- run templates/main.has --ticks 10000
cargo run --release -- bench                        # emulation speed in MHz
cargo run --release -- bench --hz 100000000         # paced at 100 MHz on the emulator thread
```

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
