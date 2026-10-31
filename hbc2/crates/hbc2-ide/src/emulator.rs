//! The emulator as the IDE sees it: the emulator thread, what is loaded in
//! it, and with which peripherals.

use std::sync::Arc;

use hbc2_asm::{Program, Target};
use hbc2_core::isa::RAM_SIZE;
use hbc2_core::peripherals::Eeprom;
use hbc2_core::peripherals::eeprom::{BOOT_RANGES, MEMORY_SIZE};
use hbc2_core::{Machine, MachineConfig};
use hbc2_runner::{Runner, Snapshot, Speed};
use serde::{Deserialize, Serialize};

/// Peripherals the user chose (the EEPROM is plugged when the program is
/// built for it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peripherals {
    pub monitor: bool,
    pub rtc: bool,
    pub keyboard: bool,
}

impl Default for Peripherals {
    fn default() -> Self {
        Peripherals {
            monitor: true,
            rtc: true,
            keyboard: true,
        }
    }
}

/// Memory image the machine starts from.
#[derive(Clone, Debug)]
pub enum Image {
    /// 64 KiB copied into RAM at power-on.
    Ram(Arc<[u8]>),
    /// 1 MiB EEPROM; it fills RAM at power-on.
    Eeprom(Arc<[u8]>),
}

impl Image {
    /// Accepts a 64 KiB RAM image or a 1 MiB EEPROM image.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Image, String> {
        match bytes.len() {
            RAM_SIZE => Ok(Image::Ram(bytes.into())),
            MEMORY_SIZE => Ok(Image::Eeprom(bytes.into())),
            n => Err(format!(
                "the file is {n} bytes; expected {RAM_SIZE} (RAM image) or {MEMORY_SIZE} (EEPROM image)"
            )),
        }
    }

    pub fn target(&self) -> Target {
        match self {
            Image::Ram(_) => Target::Ram,
            Image::Eeprom(_) => Target::Eeprom,
        }
    }
}

pub fn build_machine(image: Option<&Image>, peripherals: Peripherals) -> Result<Machine, String> {
    let mut config = MachineConfig {
        monitor: peripherals.monitor,
        rtc: peripherals.rtc,
        keyboard: peripherals.keyboard,
        eeprom: None,
    };
    let ram = match image {
        None => None,
        Some(Image::Ram(ram)) => Some(&ram[..]),
        Some(Image::Eeprom(memory)) => {
            config.eeprom = Some(Eeprom::from_image(memory.to_vec()).map_err(|e| e.to_string())?);
            None
        }
    };
    Machine::new(config, ram).map_err(|e| e.to_string())
}

/// RAM address of a program address, if the instruction there ends up in RAM.
pub fn ram_address(target: Target, address: u32) -> Option<u16> {
    let in_ram = match target {
        Target::Ram => address < RAM_SIZE as u32,
        Target::Eeprom => BOOT_RANGES.iter().any(|r| r.contains(&(address as usize))),
    };
    in_ram.then_some(address as u16)
}

pub struct Emulator {
    pub runner: Runner,
    pub peripherals: Peripherals,
    image: Option<Image>,
    /// Debug information for the image, when it was assembled here.
    pub program: Option<Arc<Program>>,
    /// State when the last run or step started, to show what changed since.
    pub baseline: Option<Arc<Snapshot>>,
}

impl Emulator {
    pub fn new(
        peripherals: Peripherals,
        speed: Speed,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Emulator {
        let machine = build_machine(None, peripherals).expect("an empty machine always builds");
        let runner = Runner::with_notify(machine, notify);
        runner.set_speed(speed);
        Emulator {
            runner,
            peripherals,
            image: None,
            program: None,
            baseline: None,
        }
    }

    pub fn image(&self) -> Option<&Image> {
        self.image.as_ref()
    }

    /// Loads a new image, paused at power-on, and sets the breakpoints.
    pub fn load(
        &mut self,
        image: Image,
        program: Option<Arc<Program>>,
        breakpoints: &[u16],
    ) -> Result<(), String> {
        let mut machine = build_machine(Some(&image), self.peripherals)?;
        for &address in breakpoints {
            machine.breakpoints.set(address, true);
        }
        self.runner.load(machine);
        self.image = Some(image);
        self.program = program;
        self.baseline = None;
        Ok(())
    }

    /// Back to an empty machine (no project).
    pub fn unload(&mut self) {
        self.image = None;
        self.program = None;
        let _ = self.rebuild(&[]);
    }

    /// Rebuilds the machine with the current image and peripherals (after
    /// the peripherals changed). Breakpoints are kept.
    pub fn rebuild(&mut self, breakpoints: &[u16]) -> Result<(), String> {
        let mut machine = build_machine(self.image.as_ref(), self.peripherals)?;
        for &address in breakpoints {
            machine.breakpoints.set(address, true);
        }
        self.runner.load(machine);
        self.baseline = None;
        Ok(())
    }

    pub fn set_breakpoints(&self, addresses: &[u16]) {
        self.runner.clear_breakpoints();
        for &address in addresses {
            self.runner.set_breakpoint(address, true);
        }
    }

    pub fn run(&mut self) {
        self.baseline = Some(self.runner.snapshot());
        self.runner.run();
    }

    pub fn pause(&self) {
        self.runner.pause();
    }

    pub fn step(&mut self, instructions: u32) {
        self.baseline = Some(self.runner.sync());
        self.runner.step(instructions);
    }

    /// Runs one instruction, then on while the PC stays on `addresses`
    /// (one source line).
    pub fn step_within(&mut self, addresses: Vec<u16>) {
        self.baseline = Some(self.runner.sync());
        self.runner.step_within(addresses);
    }

    /// Restarts the machine from its image, paused.
    pub fn reset(&mut self) {
        self.runner.power_cycle();
        self.baseline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_by_size() {
        assert!(matches!(
            Image::from_bytes(vec![0; RAM_SIZE]),
            Ok(Image::Ram(_))
        ));
        assert!(matches!(
            Image::from_bytes(vec![0; MEMORY_SIZE]),
            Ok(Image::Eeprom(_))
        ));
        assert!(Image::from_bytes(vec![0; 10]).is_err());
    }

    #[test]
    fn eeprom_breakpoints_only_in_boot_ranges() {
        assert_eq!(ram_address(Target::Ram, 0x0300), Some(0x0300));
        assert_eq!(ram_address(Target::Eeprom, 0x01FC), Some(0x01FC));
        assert_eq!(ram_address(Target::Eeprom, 0xFE04), Some(0xFE04));
        assert_eq!(ram_address(Target::Eeprom, 0x0200), None);
        assert_eq!(ram_address(Target::Eeprom, 0xF004), None);
        assert_eq!(ram_address(Target::Eeprom, 0x1_0000), None);
    }
}
