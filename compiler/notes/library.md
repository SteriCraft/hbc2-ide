# Library (step 8) — how it works

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- `compiler/lib`: `include/` (cpp gets `-I` in both drivers), `lib.has` (asm part), `src/*.c` compiled by `hbc2_cc::build_library(dir)` with units 100+ → `dir/library.has` (includes lib.has + compiled files). Programs: crt0, runtime, library.has, own files. See lib/README.md.
- `asm/screen.has`, `asm/terminal.has`: copies of the user's drivers, only the port names changed (`_monitor_port`…, header lists changes). Re-copy if the user changes the originals.
- Device detection: `runtime/devices.has` (copy of the template's port-0 handler, vector `.data` at $0xFE00, variables in `.data` not `.bss` because interrupts come before the .bss clear), crt0 waits 32 loop turns (1 was enough: power-on interrupts are taken before crt0's first instructions finish).
- Terminal from C: `lib_term_start` (hbc2.has) = the first output switches to text mode and clears (no prompt, unlike term_init; no wait for the monitor: without one output does nothing). putchar/puts in asm, getchar/gets in C (static `next`), printf in C (static `field`).
- Tests: programs.rs runs every program with monitor + RTC + keyboard (`MachineConfig::default()`), `name.screen` = expected text screen, `name.keys` = typed once `term_kbd_ready` = 1, one key per 20 000 ticks.

