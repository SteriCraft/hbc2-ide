//! Peripherals plugged into the IOD (the I/O processor's ports).
//!
//! Peripherals are event-driven: they react when the CPU writes one of their
//! ports with `OUT`, and (for time-based ones) when the host reports elapsed
//! time. They never run on every clock cycle, which keeps the CPU loop fast.

pub mod eeprom;
pub mod keyboard;
pub mod monitor;
pub mod rtc;

pub use eeprom::Eeprom;
pub use keyboard::Keyboard;
pub use monitor::Monitor;
pub use rtc::Rtc;

use crate::board::Board;

/// Device classes in INFO's descriptors: the PCI base class (high byte)
/// and subclass (low byte). Peripherals/README.md, "Device classes".
pub mod class {
    /// Display controller, VGA-compatible (`03 00`).
    pub const DISPLAY: u16 = 0x0300;
    /// Input device, keyboard (`09 00`).
    pub const KEYBOARD: u16 = 0x0900;
    /// System peripheral, real-time clock (`08 03`).
    pub const RTC: u16 = 0x0803;
    /// Mass storage, other (`01 80`): the EEPROM, an SD card.
    pub const STORAGE_OTHER: u16 = 0x0180;
}

/// Product IDs in INFO's descriptors: our own numbering, one per device
/// design (its ports and commands). 0 is never used.
pub mod product {
    pub const MONITOR: u16 = 0x0001;
    pub const RTC: u16 = 0x0002;
    pub const KEYBOARD: u16 = 0x0003;
    pub const EEPROM: u16 = 0x0004;
    /// Reserved for the SD card (not in the emulator yet).
    pub const SD_CARD: u16 = 0x0005;
}

/// Common shape of every peripheral.
pub trait Peripheral {
    /// Device class ([`class`]), sent in INFO's descriptor.
    const CLASS: u16;
    /// Product ID ([`product`]), sent in INFO's descriptor.
    const PRODUCT: u16;
    /// Number of consecutive ports the peripheral needs.
    const PORT_COUNT: u8;
    const NAME: &'static str;

    /// First allocated port, once connected.
    fn first_port(&self) -> Option<u8>;

    /// Called by the machine at power-on, after the IOD allocated the ports.
    fn power_on(&mut self, first_port: u8, board: &mut Board);

    /// The CPU wrote one of our ports. `offset` is relative to the first port.
    fn port_written(&mut self, offset: u8, board: &mut Board);

    /// Host time went by while the machine was running.
    fn advance_time(&mut self, _nanos: u64, _board: &mut Board) {}
}

/// Which peripheral owns a port, used to route `OUT` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeripheralKind {
    Monitor,
    Rtc,
    Keyboard,
    Eeprom,
}

impl PeripheralKind {
    pub const fn name(self) -> &'static str {
        match self {
            PeripheralKind::Monitor => Monitor::NAME,
            PeripheralKind::Rtc => Rtc::NAME,
            PeripheralKind::Keyboard => Keyboard::NAME,
            PeripheralKind::Eeprom => Eeprom::NAME,
        }
    }

    /// Looks up a peripheral kind from its product ID.
    pub const fn from_product(id: u16) -> Option<PeripheralKind> {
        match id {
            Monitor::PRODUCT => Some(PeripheralKind::Monitor),
            Rtc::PRODUCT => Some(PeripheralKind::Rtc),
            Keyboard::PRODUCT => Some(PeripheralKind::Keyboard),
            Eeprom::PRODUCT => Some(PeripheralKind::Eeprom),
            _ => None,
        }
    }
}

/// The peripherals plugged into a machine. `None` = not plugged.
#[derive(Clone, Debug, Default)]
pub struct Peripherals {
    pub monitor: Option<Monitor>,
    pub rtc: Option<Rtc>,
    pub keyboard: Option<Keyboard>,
    pub eeprom: Option<Eeprom>,
}

impl Peripherals {
    /// Plugs every present peripheral into the IOD (in a fixed order:
    /// monitor, RTC, keyboard, EEPROM) and powers it on. A peripheral that
    /// gets no ports is unplugged.
    pub(crate) fn power_on(&mut self, board: &mut Board) {
        fn plug<P: Peripheral>(slot: &mut Option<P>, board: &mut Board) {
            if let Some(device) = slot {
                match board.iod.connect(P::PRODUCT, P::PORT_COUNT) {
                    Some(first) => device.power_on(first, board),
                    None => *slot = None,
                }
            }
        }
        plug(&mut self.monitor, board);
        plug(&mut self.rtc, board);
        plug(&mut self.keyboard, board);
        plug(&mut self.eeprom, board);
    }

    /// Routes a CPU write on `port` to the peripheral that owns it.
    pub(crate) fn port_written(&mut self, port: u8, board: &mut Board) {
        fn deliver<P: Peripheral>(slot: &mut Option<P>, port: u8, board: &mut Board) {
            if let Some(device) = slot
                && let Some(first) = device.first_port()
            {
                device.port_written(port - first, board);
            }
        }
        match PeripheralKind::from_product(board.iod.owner(port)) {
            Some(PeripheralKind::Monitor) => deliver(&mut self.monitor, port, board),
            Some(PeripheralKind::Rtc) => deliver(&mut self.rtc, port, board),
            Some(PeripheralKind::Keyboard) => deliver(&mut self.keyboard, port, board),
            Some(PeripheralKind::Eeprom) => deliver(&mut self.eeprom, port, board),
            None => {}
        }
    }

    /// INFO's device descriptors, in port order: class, product ID (both
    /// most significant byte first), first port, number of ports.
    pub fn descriptors(&self) -> Vec<[u8; 6]> {
        fn descriptor<P: Peripheral>(slot: &Option<P>) -> Option<[u8; 6]> {
            let first = slot.as_ref()?.first_port()?;
            let [c0, c1] = P::CLASS.to_be_bytes();
            let [p0, p1] = P::PRODUCT.to_be_bytes();
            Some([c0, c1, p0, p1, first, P::PORT_COUNT])
        }
        let mut list: Vec<[u8; 6]> = [
            descriptor(&self.monitor),
            descriptor(&self.rtc),
            descriptor(&self.keyboard),
            descriptor(&self.eeprom),
        ]
        .into_iter()
        .flatten()
        .collect();
        list.sort_by_key(|d| d[4]);
        list
    }

    pub(crate) fn advance_time(&mut self, nanos: u64, board: &mut Board) {
        if let Some(rtc) = &mut self.rtc {
            rtc.advance_time(nanos, board);
        }
    }
}
