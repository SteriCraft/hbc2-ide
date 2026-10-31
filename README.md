![project_logo](https://github.com/SteriCraft/hbc2-ide/blob/main/docs/documentation/html/logo.png?raw=true)

# Home Brew Computer 2 - Integrated Development Environment
This repository hosts all of my HBC-2 project files.

It contains documentation about the computer design and an IDE built around a custom assembly language.

## Computer design
The HBC-2 is an 8-bit computer with 64 KB of RAM, a 256 * 192 pixels monitor and a real-time clock.

### Documentation
Documentation about the computer's design is available [here](https://github.com/SteriCraft/hbc2-ide/tree/main/Documentation)<br />
You can also gather information about the computer on the official blog [here](https://hbc2fr.wordpress.com/)


## Integrated Development Environment
![project_demo](hbc2/screenshot.png)

The IDE (written in Rust, in [hbc2/](hbc2/)) lets you write programs in HBC-2 assembly or in C, build them, and run them on an emulator of the computer, from 1 Hz to 100 MHz (the real CPU's target is 2 MHz). While a program runs or is paused you can see the CPU, the memory, the disassembly, the peripherals and the monitor, set breakpoints and step through the assembly or C lines.

The C compiler is based on the [Fuzix Compiler Kit](https://github.com/EtchedPixels/Fuzix-Compiler-Kit): see [compiler/](compiler/).

The previous Qt version of the IDE is in the git history.

### Documentation
How to build and use the IDE: [hbc2/README.md](hbc2/README.md). The assembly language: [HBC-2 Assembly Language](Documentation/HBC-2%20Assembly%20Language.md). Planned work: [TODO.md](TODO.md).
