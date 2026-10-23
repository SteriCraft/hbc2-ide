//! Peripherals plugged into the IOD.
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

/// Common shape of every peripheral.
pub trait Peripheral {
    /// ID sent to the CPU on port 0 when the peripheral is plugged in.
    const DEVICE_ID: u8;
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

    /// Looks up a peripheral kind from the device ID it reports.
    pub const fn from_device_id(id: u8) -> Option<PeripheralKind> {
        match id {
            Monitor::DEVICE_ID => Some(PeripheralKind::Monitor),
            Rtc::DEVICE_ID => Some(PeripheralKind::Rtc),
            Keyboard::DEVICE_ID => Some(PeripheralKind::Keyboard),
            Eeprom::DEVICE_ID => Some(PeripheralKind::Eeprom),
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
                match board.iod.connect(P::DEVICE_ID, P::PORT_COUNT) {
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
        match PeripheralKind::from_device_id(board.iod.owner(port)) {
            Some(PeripheralKind::Monitor) => deliver(&mut self.monitor, port, board),
            Some(PeripheralKind::Rtc) => deliver(&mut self.rtc, port, board),
            Some(PeripheralKind::Keyboard) => deliver(&mut self.keyboard, port, board),
            Some(PeripheralKind::Eeprom) => deliver(&mut self.eeprom, port, board),
            None => {}
        }
    }

    pub(crate) fn advance_time(&mut self, nanos: u64, board: &mut Board) {
        if let Some(rtc) = &mut self.rtc {
            rtc.advance_time(nanos, board);
        }
    }
}
