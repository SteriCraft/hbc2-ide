# HBC-2 (Rust rewrite)

New implementation of the HBC-2 emulator and IDE. The reference for its
behaviour is the HBC-2 Instruction Manual and the peripheral manuals in
`../Documentation/`. The old Qt IDE in `../HBC-2_IDE/` stays as a reference
until this one does everything it did.

## Crates

| Crate | Role |
|---|---|
| `hbc2-core` | CPU, RAM, IOD, peripherals (monitor, keyboard, RTC, EEPROM) and `Machine`. No GUI, no threads, no dependencies. |
| `hbc2-cli` | `hbc2` command: speed benchmark and headless run. |

Planned: `hbc2-asm` (assembler, disassembler), `hbc2-runner` (emulator
thread), `hbc2-ide` (egui GUI).

## Commands

```sh
cargo test                                # run all tests
cargo run --release -- bench              # emulation speed in MHz
cargo run --release -- run ram.bin --ticks 1000000
```

## Design notes

- One `Machine::tick()` is one clock cycle: it runs one whole instruction, or
  one step of the interrupt handshake, or nothing when halted.
- Peripherals are event-driven: a command runs once, when the CPU writes the
  peripheral's CMD port. Nothing runs on every cycle except the CPU and IOD.
- The machine is a plain value owned by whoever drives it. The future GUI will
  run it on one emulator thread and receive copies of its state (snapshots).
