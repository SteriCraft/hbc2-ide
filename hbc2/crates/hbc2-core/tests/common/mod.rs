#![allow(dead_code)]

use hbc2_core::isa::{IVT_START, Instruction, PROGRAM_START, RAM_SIZE, encode};
use hbc2_core::{Machine, MachineConfig, Register};

pub use hbc2_core::Opcode as O;
pub use hbc2_core::Register::*;

/// No peripherals: no plug-in interrupts, so tests see only their own program.
pub fn bare() -> MachineConfig {
    MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    }
}

/// RAM image builder.
pub struct Image(pub Vec<u8>);

impl Image {
    pub fn new() -> Image {
        Image(vec![0; RAM_SIZE])
    }

    pub fn code(mut self, address: u16, program: &[Instruction]) -> Image {
        for (n, i) in program.iter().enumerate() {
            let a = address as usize + n * 4;
            self.0[a..a + 4].copy_from_slice(&i.to_bytes());
        }
        self
    }

    pub fn program(self, program: &[Instruction]) -> Image {
        self.code(PROGRAM_START, program)
    }

    pub fn byte(mut self, address: u16, value: u8) -> Image {
        self.0[address as usize] = value;
        self
    }

    /// Sets the interrupt handler address for `port`.
    pub fn vector(mut self, port: u8, handler: u16) -> Image {
        let a = (IVT_START + port as u16 * 2) as usize;
        self.0[a..a + 2].copy_from_slice(&handler.to_be_bytes());
        self
    }

    pub fn build(self, config: MachineConfig) -> Machine {
        Machine::new(config, Some(&self.0)).expect("valid image")
    }
}

/// Runs a program on a machine without peripherals.
pub fn machine(program: &[Instruction]) -> Machine {
    Image::new().program(program).build(bare())
}

/// Executes `n` instructions.
pub fn steps(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.step();
    }
}

/// `OUT port, value`, using X and Y as scratch registers.
pub fn out(port: u8, value: u8) -> [Instruction; 3] {
    [
        encode::reg_imm8(O::Mov, X, port),
        encode::reg_imm8(O::Mov, Y, value),
        encode::reg(O::Out, X, Y),
    ]
}

/// `IN reg, port`, using X as scratch register.
pub fn input(reg: Register, port: u8) -> [Instruction; 2] {
    [
        encode::reg_imm8(O::Mov, X, port),
        encode::reg(O::In, reg, X),
    ]
}

/// Address of the n-th instruction of a program.
pub const fn at(n: u16) -> u16 {
    PROGRAM_START + n * 4
}

pub fn ram(m: &Machine, address: u16) -> u8 {
    m.board.ram.read(address)
}
