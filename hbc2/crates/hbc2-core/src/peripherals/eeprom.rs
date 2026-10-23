//! The HBC-2 EEPROM: 1 MiB of non-volatile memory (20-bit addresses).
//!
//! At power-on it copies the interrupt vector table and the start of the
//! program (0x0100-0x04FF) and the interrupt handlers (0xF000-0xFFFF) from
//! its memory into RAM.
//!
//! Ports: CMD, DATA, ADDR_0 (bits 19-16), ADDR_1 (bits 15-8), ADDR_2 (bits 7-0).
//! Writing CMD executes READ (DATA = memory[address]) or WRITE
//! (memory[address] = DATA).

use super::Peripheral;
use crate::board::Board;

pub const MEMORY_SIZE: usize = 0x10_0000;

pub mod port {
    pub const CMD: u8 = 0;
    pub const DATA: u8 = 1;
    pub const ADDR_0: u8 = 2;
    pub const ADDR_1: u8 = 3;
    pub const ADDR_2: u8 = 4;
}

pub mod command {
    pub const NOP: u8 = 0;
    pub const READ: u8 = 1;
    pub const WRITE: u8 = 2;
}

/// RAM ranges copied from the EEPROM at power-on.
pub const BOOT_RANGES: [std::ops::Range<usize>; 2] = [0x0100..0x0500, 0xF000..0x1_0000];

#[derive(Clone)]
pub struct Eeprom {
    first_port: Option<u8>,
    memory: Box<[u8]>,
    /// Incremented on every write, so viewers can skip copying the memory.
    generation: u64,
}

impl std::fmt::Debug for Eeprom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Eeprom")
            .field("first_port", &self.first_port)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrongSize(pub usize);

impl std::fmt::Display for WrongSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EEPROM image is {} bytes, expected {} bytes",
            self.0, MEMORY_SIZE
        )
    }
}

impl std::error::Error for WrongSize {}

impl Eeprom {
    /// An EEPROM filled with zeros.
    pub fn blank() -> Eeprom {
        Eeprom {
            first_port: None,
            memory: vec![0; MEMORY_SIZE].into_boxed_slice(),
            generation: 0,
        }
    }

    /// An EEPROM loaded with an image of exactly 1 MiB.
    pub fn from_image(image: Vec<u8>) -> Result<Eeprom, WrongSize> {
        if image.len() != MEMORY_SIZE {
            return Err(WrongSize(image.len()));
        }
        Ok(Eeprom {
            first_port: None,
            memory: image.into_boxed_slice(),
            generation: 0,
        })
    }

    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl Peripheral for Eeprom {
    const DEVICE_ID: u8 = 0xE1;
    const PORT_COUNT: u8 = 5;
    const NAME: &'static str = "EEPROM";

    fn first_port(&self) -> Option<u8> {
        self.first_port
    }

    /// Memory content is kept: the EEPROM is non-volatile.
    fn power_on(&mut self, first_port: u8, board: &mut Board) {
        self.first_port = Some(first_port);
        let ram = board.ram.as_mut_slice();
        for range in BOOT_RANGES {
            ram[range.clone()].copy_from_slice(&self.memory[range]);
        }
    }

    fn port_written(&mut self, offset: u8, board: &mut Board) {
        let Some(first) = self.first_port else {
            return;
        };
        if offset != port::CMD {
            return;
        }
        let iod = &mut board.iod;
        let address = ((iod.port(first + port::ADDR_0) as usize & 0x0F) << 16)
            | ((iod.port(first + port::ADDR_1) as usize) << 8)
            | iod.port(first + port::ADDR_2) as usize;

        match iod.port(first + port::CMD) {
            command::READ => iod.set_port(first + port::DATA, self.memory[address]),
            command::WRITE => {
                self.memory[address] = iod.port(first + port::DATA);
                self.generation += 1;
            }
            _ => {}
        }
    }
}
