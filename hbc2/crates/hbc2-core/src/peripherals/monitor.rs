//! The HBC-2 Monitor (see the HBC-2 Monitor Instruction Manual).
//!
//! Ports: DATA_0, DATA_1, POS_X, POS_Y, CMD. Writing CMD executes the command
//! using the values currently latched on the other ports.

use super::Peripheral;
use crate::board::Board;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 192;
pub const CHAR_WIDTH: usize = 6;
pub const CHAR_HEIGHT: usize = 8;
pub const TEXT_COLUMNS: usize = 42;
pub const TEXT_ROWS: usize = 24;
pub const PIXEL_BUFFER_SIZE: usize = WIDTH * HEIGHT;
pub const TEXT_BUFFER_SIZE: usize = TEXT_COLUMNS * TEXT_ROWS;
/// First displayable ASCII code; 95 characters (32 to 126) are displayable.
pub const FIRST_DISPLAYABLE_CHAR: u8 = 0x20;
pub const DISPLAYABLE_CHARS: u8 = 95;

/// The 16-colour palette, as 0xRRGGBB.
pub const PALETTE: [u32; 16] = [
    0x000000, // 0x0 Black
    0x0000AA, // 0x1 Blue
    0x00AA00, // 0x2 Green
    0x00AAAA, // 0x3 Cyan
    0xAA0000, // 0x4 Red
    0xAA00AA, // 0x5 Magenta
    0xAA5500, // 0x6 Brown
    0xAAAAAA, // 0x7 Light gray
    0x555555, // 0x8 Bright black
    0x5555FF, // 0x9 Bright blue
    0x55FF55, // 0xA Bright green
    0x55FFFF, // 0xB Bright cyan
    0xFF5555, // 0xC Bright red
    0xFF55FF, // 0xD Bright magenta
    0xFFFF55, // 0xE Bright yellow
    0xFFFFFF, // 0xF White
];

pub mod port {
    pub const DATA_0: u8 = 0;
    pub const DATA_1: u8 = 1;
    pub const POS_X: u8 = 2;
    pub const POS_Y: u8 = 3;
    pub const CMD: u8 = 4;
}

pub mod command {
    pub const NOP: u8 = 0;
    pub const WRITE: u8 = 1;
    pub const READ: u8 = 2;
    pub const SWITCH_TO_PIXEL_MODE: u8 = 3;
    pub const SWITCH_TO_TEXT_MODE: u8 = 4;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Pixel = 0,
    Text = 1,
}

/// One character cell in text mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextCell {
    /// High nibble: background colour. Low nibble: character colour.
    pub colors: u8,
    pub ascii: u8,
}

impl TextCell {
    pub fn background(self) -> u8 {
        self.colors >> 4
    }
    pub fn foreground(self) -> u8 {
        self.colors & 0x0F
    }
}

#[derive(Clone)]
pub struct Monitor {
    first_port: Option<u8>,
    mode: Mode,
    text: Box<[TextCell; TEXT_BUFFER_SIZE]>,
    /// One pixel per byte (colour index in the low nibble). The manual's
    /// hardware packs 2 pixels per byte; the IDE keeps one per byte for simplicity.
    pixels: Box<[u8; PIXEL_BUFFER_SIZE]>,
    /// Incremented on every change, so viewers can skip redrawing.
    generation: u64,
}

impl Default for Monitor {
    fn default() -> Self {
        Monitor::new()
    }
}

impl std::fmt::Debug for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Monitor")
            .field("first_port", &self.first_port)
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {
            first_port: None,
            mode: Mode::Text,
            text: Box::new([TextCell::default(); TEXT_BUFFER_SIZE]),
            pixels: Box::new([0; PIXEL_BUFFER_SIZE]),
            generation: 0,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn text_buffer(&self) -> &[TextCell; TEXT_BUFFER_SIZE] {
        &self.text
    }

    pub fn pixel_buffer(&self) -> &[u8; PIXEL_BUFFER_SIZE] {
        &self.pixels
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn pixel_index(x: u8, y: u8) -> Option<usize> {
        ((y as usize) < HEIGHT).then(|| x as usize + y as usize * WIDTH)
    }

    fn text_index(x: u8, y: u8) -> Option<usize> {
        ((x as usize) < TEXT_COLUMNS && (y as usize) < TEXT_ROWS)
            .then(|| x as usize + y as usize * TEXT_COLUMNS)
    }

    fn execute(&mut self, first: u8, board: &mut Board) {
        let iod = &mut board.iod;
        let get = |offset: u8| iod.port(first + offset);
        let (data_0, data_1) = (get(port::DATA_0), get(port::DATA_1));
        let (x, y) = (get(port::POS_X), get(port::POS_Y));

        match iod.port(first + port::CMD) {
            command::WRITE => match self.mode {
                Mode::Pixel => {
                    if let Some(index) = Self::pixel_index(x, y) {
                        self.pixels[index] = data_0 & 0x0F;
                        self.generation += 1;
                    }
                }
                Mode::Text => {
                    if let Some(index) = Self::text_index(x, y) {
                        self.text[index] = TextCell {
                            colors: data_0,
                            ascii: data_1,
                        };
                        self.generation += 1;
                    }
                }
            },
            command::READ => {
                // Invalid positions read as 0x00.
                let (d0, d1) = match self.mode {
                    Mode::Pixel => (Self::pixel_index(x, y).map_or(0, |i| self.pixels[i]), 0),
                    Mode::Text => Self::text_index(x, y)
                        .map_or((0, 0), |i| (self.text[i].colors, self.text[i].ascii)),
                };
                iod.set_port(first + port::DATA_0, d0);
                iod.set_port(first + port::DATA_1, d1);
            }
            command::SWITCH_TO_PIXEL_MODE => {
                self.mode = Mode::Pixel;
                self.generation += 1;
            }
            command::SWITCH_TO_TEXT_MODE => {
                self.mode = Mode::Text;
                self.generation += 1;
            }
            _ => {} // NOP and invalid commands
        }
    }
}

impl Peripheral for Monitor {
    const DEVICE_ID: u8 = 0x4A;
    const PORT_COUNT: u8 = 5;
    const NAME: &'static str = "Monitor";

    fn first_port(&self) -> Option<u8> {
        self.first_port
    }

    fn power_on(&mut self, first_port: u8, _board: &mut Board) {
        *self = Monitor::new();
        self.first_port = Some(first_port);
    }

    fn port_written(&mut self, offset: u8, board: &mut Board) {
        if offset == port::CMD
            && let Some(first) = self.first_port
        {
            self.execute(first, board);
        }
    }
}
