//! The HBC-2 keyboard.
//!
//! When a key is pressed, the keyboard sends an interrupt on its first port
//! with the scan code as data. When a key is released, the same happens on
//! its second port. Mapping host keys to scan codes is the front end's job.

use super::Peripheral;
use crate::board::Board;

pub mod port {
    pub const PRESSED_SCAN_CODE: u8 = 0;
    pub const RELEASED_SCAN_CODE: u8 = 1;
}

#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    first_port: Option<u8>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard::default()
    }

    /// Sends a key event to the CPU. Returns false if the interrupt was
    /// dropped (keyboard not connected, or IOD queue full).
    pub fn key_event(&mut self, scan_code: u8, pressed: bool, board: &mut Board) -> bool {
        let Some(first) = self.first_port else {
            return false;
        };
        let port = first
            + if pressed {
                port::PRESSED_SCAN_CODE
            } else {
                port::RELEASED_SCAN_CODE
            };
        board.iod.set_port(port, scan_code);
        board.iod.trigger_interrupt(port)
    }
}

impl Peripheral for Keyboard {
    const DEVICE_ID: u8 = 0xAA;
    const PORT_COUNT: u8 = 2;
    const NAME: &'static str = "Keyboard";

    fn first_port(&self) -> Option<u8> {
        self.first_port
    }

    fn power_on(&mut self, first_port: u8, _board: &mut Board) {
        self.first_port = Some(first_port);
    }

    fn port_written(&mut self, _offset: u8, _board: &mut Board) {}
}
