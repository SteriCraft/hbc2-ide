//! The HBC-2 screen: the native display of the I/O processor
//! (Documentation/Peripherals/Native/Reference.md), and the old monitor
//! ports mapped onto it (Documentation/Peripherals/Monitor/Reference.md).
//!
//! One picture, 256 × 192 pixels of 16 colours. Text is drawn into it with
//! an 8 × 8 font (Windows-1252, a RAM copy that DEFCHAR changes), on a grid
//! of 32 × 24 text cells; the cells remember the character and its colours
//! for GET_CHAR and SCROLL.
//!
//! The old monitor ports: DATA_0, DATA_1, POS_X, POS_Y, CMD. Writing CMD
//! runs the command with the values the other ports hold. Their text mode
//! writes and reads the native text cells, their pixel mode the pixels.

use super::Peripheral;
use super::font::DEFAULT_FONT;
use crate::board::Board;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 192;
/// A text cell is 8 × 8 pixels.
pub const CELL: usize = 8;
pub const TEXT_COLUMNS: usize = WIDTH / CELL;
pub const TEXT_ROWS: usize = HEIGHT / CELL;
pub const PIXEL_BUFFER_SIZE: usize = WIDTH * HEIGHT;
pub const TEXT_BUFFER_SIZE: usize = TEXT_COLUMNS * TEXT_ROWS;

/// Text colours at power-on, after the reset button and `TEXT_RESET`:
/// bright yellow on blue.
pub const DEFAULT_FG: u8 = 0xE;
pub const DEFAULT_BG: u8 = 0x1;

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

/// The old monitor ports, by offset from the first one.
pub mod port {
    pub const DATA_0: u8 = 0;
    pub const DATA_1: u8 = 1;
    pub const POS_X: u8 = 2;
    pub const POS_Y: u8 = 3;
    pub const CMD: u8 = 4;
}

/// The old monitor commands (written on CMD).
pub mod command {
    pub const NOP: u8 = 0;
    pub const WRITE: u8 = 1;
    pub const READ: u8 = 2;
    pub const SWITCH_TO_PIXEL_MODE: u8 = 3;
    pub const SWITCH_TO_TEXT_MODE: u8 = 4;
}

/// What the old monitor ports' WRITE and READ reach. Nothing changes on
/// screen when it switches: there is one picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Pixel = 0,
    Text = 1,
}

/// One text cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextCell {
    /// High nibble: background colour. Low nibble: character colour.
    pub colors: u8,
    /// The character code (Windows-1252).
    pub code: u8,
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
    /// The native display's port (its descriptor, the VSYNC event).
    display_port: Option<u8>,
    mode: Mode,
    /// One pixel per byte, the colour index (0-15).
    pixels: Box<[u8; PIXEL_BUFFER_SIZE]>,
    cells: Box<[TextCell; TEXT_BUFFER_SIZE]>,
    font: Box<[[u8; 8]; 256]>,
    cursor_x: u8,
    cursor_y: u8,
    /// A character was drawn on the last column: the next one wraps first.
    pending_wrap: bool,
    fg: u8,
    bg: u8,
    cursor_shown: bool,
    vsync: bool,
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
            .field("display_port", &self.display_port)
            .field("mode", &self.mode)
            .field("cursor", &self.cursor())
            .finish_non_exhaustive()
    }
}

impl Monitor {
    /// The screen at power-on: blue, spaces, default font and colours.
    pub fn new() -> Monitor {
        let blank = TextCell {
            colors: DEFAULT_BG << 4 | DEFAULT_FG,
            code: b' ',
        };
        Monitor {
            first_port: None,
            display_port: None,
            mode: Mode::Text,
            pixels: Box::new([DEFAULT_BG; PIXEL_BUFFER_SIZE]),
            cells: Box::new([blank; TEXT_BUFFER_SIZE]),
            font: Box::new(DEFAULT_FONT),
            cursor_x: 0,
            cursor_y: 0,
            pending_wrap: false,
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            cursor_shown: false,
            vsync: false,
            generation: 0,
        }
    }

    // --- State, for viewers and tests ---------------------------------

    /// What the old ports' WRITE and READ reach.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn pixel_buffer(&self) -> &[u8; PIXEL_BUFFER_SIZE] {
        &self.pixels
    }

    pub fn text_buffer(&self) -> &[TextCell; TEXT_BUFFER_SIZE] {
        &self.cells
    }

    pub fn font(&self) -> &[[u8; 8]; 256] {
        &self.font
    }

    /// The cursor's cell, and whether a wrap is pending.
    pub fn cursor(&self) -> (u8, u8, bool) {
        (self.cursor_x, self.cursor_y, self.pending_wrap)
    }

    /// The text colours (character, background).
    pub fn colors(&self) -> (u8, u8) {
        (self.fg, self.bg)
    }

    /// SHOW_CURSOR: the firmware blinks an underline at the cursor.
    pub fn cursor_shown(&self) -> bool {
        self.cursor_shown
    }

    /// VSYNC events on.
    pub fn vsync(&self) -> bool {
        self.vsync
    }

    pub fn display_port(&self) -> Option<u8> {
        self.display_port
    }

    pub(crate) fn set_display_port(&mut self, port: Option<u8>) {
        self.display_port = port;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The reset button (`rst_n`): default font, colours and cursor, VSYNC
    /// off, the old ports back in text mode. The picture stays.
    pub fn reset_button(&mut self) {
        self.default_text_state();
        self.vsync = false;
        self.mode = Mode::Text;
        self.generation += 1;
    }

    fn default_text_state(&mut self) {
        *self.font = DEFAULT_FONT;
        self.fg = DEFAULT_FG;
        self.bg = DEFAULT_BG;
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.pending_wrap = false;
        self.cursor_shown = false;
    }

    // --- Drawing -------------------------------------------------------

    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        if x < WIDTH && y < HEIGHT {
            self.pixels[x + y * WIDTH] = color & 0x0F;
        }
    }

    /// Draws `code` in cell (x, y) with `colors` (`bg << 4 | fg`) and
    /// stores it in the cell. A control code is stored and drawn blank.
    fn draw_cell(&mut self, x: u8, y: u8, code: u8, colors: u8) {
        let (x, y) = (x as usize, y as usize);
        self.cells[x + y * TEXT_COLUMNS] = TextCell { colors, code };
        let glyph = if code < 0x20 {
            [0; 8]
        } else {
            self.font[code as usize]
        };
        let (fg, bg) = (colors & 0x0F, colors >> 4);
        for (row, bits) in glyph.iter().enumerate() {
            let start = (y * CELL + row) * WIDTH + x * CELL;
            for col in 0..CELL {
                let on = bits & (0x80 >> col) != 0;
                self.pixels[start + col] = if on { fg } else { bg };
            }
        }
    }

    fn text_colors(&self) -> u8 {
        self.bg << 4 | self.fg
    }

    /// Moves the cursor to the start of the next row, scrolling on the last.
    fn new_line(&mut self) {
        self.pending_wrap = false;
        self.cursor_x = 0;
        if self.cursor_y as usize == TEXT_ROWS - 1 {
            self.scroll(1);
        } else {
            self.cursor_y += 1;
        }
    }

    // --- Native text commands -----------------------------------------

    /// PUTC: draws `c` at the cursor, or runs control code `c`.
    pub fn putc(&mut self, c: u8) {
        let last_column = (TEXT_COLUMNS - 1) as u8;
        match c {
            0x08 => {
                if self.pending_wrap {
                    self.pending_wrap = false;
                } else if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = last_column;
                }
            }
            0x09 => {
                if self.pending_wrap || self.cursor_x as usize >= TEXT_COLUMNS - 8 {
                    self.new_line();
                } else {
                    self.cursor_x = (self.cursor_x / 8 + 1) * 8;
                }
            }
            0x0A => self.new_line(),
            0x0D => {
                self.cursor_x = 0;
                self.pending_wrap = false;
            }
            0x00..=0x1F => {}
            _ => {
                if self.pending_wrap {
                    self.new_line();
                }
                self.draw_cell(self.cursor_x, self.cursor_y, c, self.text_colors());
                if self.cursor_x == last_column {
                    self.pending_wrap = true;
                } else {
                    self.cursor_x += 1;
                }
            }
        }
        self.generation += 1;
    }

    /// PUTS: [`Monitor::putc`] for each character.
    pub fn puts(&mut self, text: &[u8]) {
        for &c in text {
            self.putc(c);
        }
    }

    /// CURSOR: ignored off the grid.
    pub fn set_cursor(&mut self, x: u8, y: u8) {
        if (x as usize) < TEXT_COLUMNS && (y as usize) < TEXT_ROWS {
            self.cursor_x = x;
            self.cursor_y = y;
            self.pending_wrap = false;
            self.generation += 1;
        }
    }

    /// COLOR.
    pub fn set_colors(&mut self, fg: u8, bg: u8) {
        self.fg = fg & 0x0F;
        self.bg = bg & 0x0F;
    }

    /// CLEAR: the screen in `bg`, spaces in (fg, bg), cursor home.
    pub fn clear(&mut self, bg: u8) {
        let bg = bg & 0x0F;
        self.pixels.fill(bg);
        self.cells.fill(TextCell {
            colors: bg << 4 | self.fg,
            code: b' ',
        });
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.pending_wrap = false;
        self.generation += 1;
    }

    /// SCROLL: the screen moves up `n` rows; the rows freed at the bottom
    /// are cleared in the text background. The cursor stays.
    pub fn scroll(&mut self, n: u8) {
        let n = (n as usize).min(TEXT_ROWS);
        if n == 0 {
            return;
        }
        let kept_rows = TEXT_ROWS - n;
        self.pixels.copy_within(n * CELL * WIDTH.., 0);
        self.pixels[kept_rows * CELL * WIDTH..].fill(self.bg);
        self.cells.copy_within(n * TEXT_COLUMNS.., 0);
        let blank = TextCell {
            colors: self.text_colors(),
            code: b' ',
        };
        self.cells[kept_rows * TEXT_COLUMNS..].fill(blank);
        self.generation += 1;
    }

    /// DEFCHAR: a control code is ignored.
    pub fn define_char(&mut self, code: u8, rows: [u8; 8]) {
        if code >= 0x20 {
            self.font[code as usize] = rows;
        }
    }

    /// GET_CHAR: (character, colours), (0, 0) off the grid.
    pub fn get_char(&self, x: u8, y: u8) -> (u8, u8) {
        if (x as usize) < TEXT_COLUMNS && (y as usize) < TEXT_ROWS {
            let cell = self.cells[x as usize + y as usize * TEXT_COLUMNS];
            (cell.code, cell.colors)
        } else {
            (0, 0)
        }
    }

    /// GET_CURSOR: (column, row); the column is 32 with a pending wrap.
    pub fn get_cursor(&self) -> (u8, u8) {
        let x = if self.pending_wrap {
            TEXT_COLUMNS as u8
        } else {
            self.cursor_x
        };
        (x, self.cursor_y)
    }

    /// SHOW_CURSOR.
    pub fn show_cursor(&mut self, on: bool) {
        self.cursor_shown = on;
        self.generation += 1;
    }

    /// TEXT_RESET: default font, colours and cursor, the screen cleared.
    pub fn text_reset(&mut self) {
        self.default_text_state();
        self.clear(DEFAULT_BG);
    }

    // --- Native graphics commands -------------------------------------

    /// PIXEL.
    pub fn pixel(&mut self, x: u8, y: u8, color: u8) {
        self.set_pixel(x as usize, y as usize, color);
        self.generation += 1;
    }

    /// LINE: Bresenham, both end points drawn, clipped. Drawn from the
    /// leftmost end (the top one when vertical), so a line gives the same
    /// pixels whichever end comes first.
    pub fn line(&mut self, x0: u8, y0: u8, x1: u8, y1: u8, color: u8) {
        let ((x0, y0), (x1, y1)) = if (x1, y1) < (x0, y0) {
            ((x1, y1), (x0, y0))
        } else {
            ((x0, y0), (x1, y1))
        };
        let (mut x, mut y) = (x0 as i32, y0 as i32);
        let (x1, y1) = (x1 as i32, y1 as i32);
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set_pixel(x as usize, y as usize, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        self.generation += 1;
    }

    /// RECT: the outline of w × h pixels from (x, y), clipped.
    pub fn rect(&mut self, x: u8, y: u8, w: u8, h: u8, color: u8) {
        if w == 0 || h == 0 {
            return;
        }
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = (x0 + w as usize - 1, y0 + h as usize - 1);
        for px in x0..=x1 {
            self.set_pixel(px, y0, color);
            self.set_pixel(px, y1, color);
        }
        for py in y0..=y1 {
            self.set_pixel(x0, py, color);
            self.set_pixel(x1, py, color);
        }
        self.generation += 1;
    }

    /// FILL_RECT: w × h pixels from (x, y), clipped.
    pub fn fill_rect(&mut self, x: u8, y: u8, w: u8, h: u8, color: u8) {
        let x0 = x as usize;
        let x1 = (x0 + w as usize).min(WIDTH);
        for py in y as usize..(y as usize + h as usize).min(HEIGHT) {
            self.pixels[py * WIDTH + x0..py * WIDTH + x1].fill(color & 0x0F);
        }
        self.generation += 1;
    }

    /// GET_PIXEL: the colour, 0 off screen.
    pub fn get_pixel(&self, x: u8, y: u8) -> u8 {
        if (y as usize) < HEIGHT {
            self.pixels[x as usize + y as usize * WIDTH]
        } else {
            0
        }
    }

    /// VSYNC.
    pub fn set_vsync(&mut self, on: bool) {
        self.vsync = on;
    }

    // --- The old monitor ports ----------------------------------------

    fn execute(&mut self, first: u8, board: &mut Board) {
        let iod = &mut board.iod;
        let get = |offset: u8| iod.port(first + offset);
        let (data_0, data_1) = (get(port::DATA_0), get(port::DATA_1));
        let (x, y) = (get(port::POS_X), get(port::POS_Y));

        match iod.port(first + port::CMD) {
            command::WRITE => match self.mode {
                Mode::Pixel => self.pixel(x, y, data_0),
                Mode::Text => {
                    if (x as usize) < TEXT_COLUMNS && (y as usize) < TEXT_ROWS {
                        self.draw_cell(x, y, data_1, data_0);
                        self.generation += 1;
                    }
                }
            },
            command::READ => {
                let (d0, d1) = match self.mode {
                    Mode::Pixel => (self.get_pixel(x, y), 0),
                    Mode::Text => {
                        let (code, colors) = self.get_char(x, y);
                        (colors, code)
                    }
                };
                iod.set_port(first + port::DATA_0, d0);
                iod.set_port(first + port::DATA_1, d1);
            }
            command::SWITCH_TO_PIXEL_MODE => self.mode = Mode::Pixel,
            command::SWITCH_TO_TEXT_MODE => self.mode = Mode::Text,
            _ => {} // NOP and invalid commands
        }
    }
}

impl Peripheral for Monitor {
    const CLASS: u16 = super::class::DISPLAY;
    const PRODUCT: u16 = super::product::MONITOR;
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
