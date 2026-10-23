//! The motherboard: RAM, the IOD chip, the buses and the interrupt control lines.

use crate::iod::Iod;
use crate::isa::RAM_SIZE;

/// 64 KiB of RAM.
#[derive(Clone)]
pub struct Ram(Box<[u8; RAM_SIZE]>);

impl Ram {
    pub fn new() -> Ram {
        Ram(Box::new([0; RAM_SIZE]))
    }

    #[inline(always)]
    pub fn read(&self, address: u16) -> u8 {
        self.0[address as usize]
    }

    #[inline(always)]
    pub fn write(&mut self, address: u16, value: u8) {
        self.0[address as usize] = value;
    }

    /// Reads a big-endian 16-bit value (MSB at `address`).
    #[inline(always)]
    pub fn read_u16(&self, address: u16) -> u16 {
        u16::from_be_bytes([self.read(address), self.read(address.wrapping_add(1))])
    }

    /// Reads a big-endian 32-bit instruction word.
    #[inline(always)]
    pub fn read_u32(&self, address: u16) -> u32 {
        u32::from_be_bytes([
            self.read(address),
            self.read(address.wrapping_add(1)),
            self.read(address.wrapping_add(2)),
            self.read(address.wrapping_add(3)),
        ])
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }

    pub fn fill(&mut self, value: u8) {
        self.0.fill(value);
    }
}

impl Default for Ram {
    fn default() -> Self {
        Ram::new()
    }
}

impl std::fmt::Debug for Ram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ram({} bytes)", RAM_SIZE)
    }
}

/// Everything the CPU is wired to.
#[derive(Clone, Debug, Default)]
pub struct Board {
    pub ram: Ram,
    pub iod: Iod,
    /// Last value placed on the address bus by an I/O access or an interrupt.
    pub address_bus: u16,
    /// Last value placed on the data bus by an I/O access or an interrupt.
    pub data_bus: u8,
    /// INT line: an interrupt is pending (driven by the IOD).
    pub int: bool,
    /// INR line: the CPU is ready to receive the interrupt (driven by the CPU).
    pub inr: bool,
}

impl Board {
    /// Resets buses, lines and the IOD. RAM is left untouched.
    pub fn reset(&mut self) {
        self.address_bus = 0;
        self.data_bus = 0;
        self.int = false;
        self.inr = false;
        self.iod.reset();
    }

    /// One clock cycle of the IOD side of the interrupt handshake
    /// ("Hardware interrupts" in the manual).
    #[inline(always)]
    pub fn tick_iod(&mut self) {
        if !self.inr {
            // Step 1 / 5: raise INT while the queue is not empty.
            self.int = self.iod.has_pending_interrupt();
        } else {
            // Step 3: the CPU is ready. Lower INT and put port number and data on the buses.
            self.int = false;
            if let Some(interrupt) = self.iod.pop_interrupt() {
                self.address_bus = interrupt.port as u16;
                self.data_bus = interrupt.data;
            }
        }
    }
}
