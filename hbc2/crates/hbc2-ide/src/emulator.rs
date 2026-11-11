//! The emulator as the IDE sees it: the emulator thread, what is loaded in
//! it, and with which peripherals.

use std::sync::Arc;

use hbc2_asm::Program;
use hbc2_core::isa::RAM_SIZE;
use hbc2_core::peripherals::Eeprom;
use hbc2_core::{IopConfig, Machine, MachineConfig};
use hbc2_runner::{Runner, Snapshot, Speed};
use serde::{Deserialize, Serialize};

/// Peripherals the user chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Peripherals {
    pub monitor: bool,
    pub rtc: bool,
    pub keyboard: bool,
    /// The EEPROM (storage). Its content is kept from one machine to the
    /// next (rebuilds, new programs), as on the hardware.
    pub eeprom: bool,
}

impl Default for Peripherals {
    fn default() -> Self {
        Peripherals {
            monitor: true,
            rtc: true,
            keyboard: true,
            eeprom: true,
        }
    }
}

/// The 64 KiB RAM image the machine starts from (the boot ROM's content on
/// the hardware).
#[derive(Clone, Debug)]
pub struct Image(pub Arc<[u8]>);

impl Image {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Image, String> {
        match bytes.len() {
            RAM_SIZE => Ok(Image(bytes.into())),
            n => Err(format!(
                "the file is {n} bytes; expected {RAM_SIZE} (a RAM image)"
            )),
        }
    }
}

/// `eeprom`: the EEPROM's content, when it has one already (else blank).
pub fn build_machine(
    image: Option<&Image>,
    peripherals: Peripherals,
    eeprom: Option<Eeprom>,
) -> Result<Machine, String> {
    let config = MachineConfig {
        monitor: peripherals.monitor,
        rtc: peripherals.rtc,
        keyboard: peripherals.keyboard,
        eeprom: peripherals
            .eeprom
            .then(|| eeprom.unwrap_or_else(Eeprom::blank)),
        mailbox: Some(IopConfig::default()),
    };
    Machine::new(config, image.map(|i| &i.0[..])).map_err(|e| e.to_string())
}

/// RAM address of a program address.
pub fn ram_address(address: u32) -> Option<u16> {
    (address < RAM_SIZE as u32).then_some(address as u16)
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
        let machine =
            build_machine(None, peripherals, None).expect("an empty machine always builds");
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

    /// The EEPROM's content now, for the next machine.
    fn eeprom(&self) -> Option<Eeprom> {
        self.runner.sync().eeprom.as_deref().cloned()
    }

    /// Replaces the EEPROM's content (a 1 MiB file), on a new machine.
    pub fn load_eeprom(&mut self, content: Vec<u8>, breakpoints: &[u16]) -> Result<(), String> {
        let eeprom = Eeprom::from_image(content).map_err(|e| e.to_string())?;
        let mut machine = build_machine(self.image.as_ref(), self.peripherals, Some(eeprom))?;
        for &address in breakpoints {
            machine.breakpoints.set(address, true);
        }
        self.runner.load(machine);
        self.baseline = None;
        Ok(())
    }

    /// Loads a new image, paused at power-on, and sets the breakpoints.
    pub fn load(
        &mut self,
        image: Image,
        program: Option<Arc<Program>>,
        breakpoints: &[u16],
    ) -> Result<(), String> {
        let mut machine = build_machine(Some(&image), self.peripherals, self.eeprom())?;
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
        let mut machine = build_machine(self.image.as_ref(), self.peripherals, self.eeprom())?;
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

    /// Like `step_within`, but the routines called run until they return.
    pub fn step_over(&mut self, addresses: Vec<u16>) {
        self.baseline = Some(self.runner.sync());
        self.runner.step_over(addresses);
    }

    /// Runs until the current routine returns.
    pub fn step_out(&mut self) {
        self.baseline = Some(self.runner.sync());
        self.runner.step_out();
    }

    /// The reset button: the CPU starts again from the image, paused. The
    /// I/O processor and the peripherals keep running.
    pub fn reset(&mut self) {
        self.runner.reset();
        self.baseline = None;
    }

    /// Power off and on: the whole machine starts again from its image, paused.
    pub fn restart(&mut self) {
        self.runner.power_cycle();
        self.baseline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_by_size() {
        assert!(Image::from_bytes(vec![0; RAM_SIZE]).is_ok());
        assert!(Image::from_bytes(vec![0; 0x10_0000]).is_err());
        assert!(Image::from_bytes(vec![0; 10]).is_err());
        assert_eq!(ram_address(0x0300), Some(0x0300));
        assert_eq!(ram_address(0x1_0000), None);
    }

    #[test]
    fn eeprom_content_is_kept_across_machines() {
        let mut emulator = Emulator::new(Peripherals::default(), Speed::Fastest, || {});
        let mut content = vec![0; 0x10_0000];
        content[0x1234] = 0x42;
        emulator.load_eeprom(content, &[]).unwrap();
        emulator
            .load(Image(vec![0; RAM_SIZE].into()), None, &[])
            .unwrap();
        let eeprom = emulator.runner.sync().eeprom.clone().expect("plugged");
        assert_eq!(eeprom.memory()[0x1234], 0x42);
    }
}
