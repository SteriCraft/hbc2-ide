//! The native display (Peripherals/Native/Reference.md): the screen's
//! behaviour, command by command, on the `Monitor`, and the VSYNC event
//! through the machine. The byte protocol itself is tested from assembly
//! in hbc2-asm/tests/native.rs.

mod common;

use common::*;
use hbc2_core::isa::Instruction;
use hbc2_core::isa::encode::*;
use hbc2_core::machine::{CPU_HZ, FRAME_RATE, frame_start};
use hbc2_core::peripherals::Monitor;
use hbc2_core::peripherals::font::DEFAULT_FONT;
use hbc2_core::peripherals::monitor::{
    CELL, DEFAULT_BG, DEFAULT_FG, HEIGHT, TEXT_COLUMNS, TEXT_ROWS, WIDTH,
};
use hbc2_core::{AddrMode, Machine, MachineConfig, Register};

/// An instruction on one register (psh, pop, inc).
fn one(op: O, r: Register) -> Instruction {
    raw(op, AddrMode::Reg, r as u8, 0, 0, 0)
}

fn char_at(m: &Monitor, x: u8, y: u8) -> u8 {
    m.get_char(x, y).0
}

fn row_text(m: &Monitor, y: u8) -> String {
    (0..TEXT_COLUMNS as u8)
        .map(|x| char_at(m, x, y) as char)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The 8 × 8 pixels of cell (x, y), as glyph rows (bit 7 = left), with
/// `fg` pixels as 1 and anything else as 0.
fn cell_bits(m: &Monitor, x: usize, y: usize, fg: u8) -> [u8; 8] {
    let mut rows = [0; 8];
    for (row, bits) in rows.iter_mut().enumerate() {
        for col in 0..CELL {
            let p = m.pixel_buffer()[(y * CELL + row) * WIDTH + x * CELL + col];
            if p == fg {
                *bits |= 0x80 >> col;
            }
        }
    }
    rows
}

fn fresh() -> Monitor {
    let mut m = Monitor::new();
    m.clear(0);
    m.set_colors(15, 0);
    m
}

#[test]
fn power_on_is_blue_with_yellow_text() {
    let m = Monitor::new();
    assert!(m.pixel_buffer().iter().all(|&p| p == DEFAULT_BG));
    assert_eq!((DEFAULT_FG, DEFAULT_BG), (14, 1), "bright yellow on blue");
    assert_eq!(m.colors(), (14, 1));
    assert_eq!(m.get_char(0, 0), (b' ', 0x1E));
    assert_eq!(m.get_cursor(), (0, 0));
    assert!(!m.cursor_shown() && !m.vsync());
    assert_eq!(*m.font(), DEFAULT_FONT);
}

#[test]
fn putc_draws_the_glyph_and_the_cell() {
    let mut m = fresh();
    m.set_colors(0xC, 0x1);
    m.putc(b'A');
    assert_eq!(m.get_char(0, 0), (b'A', 0x1C));
    assert_eq!(cell_bits(&m, 0, 0, 0xC), DEFAULT_FONT[b'A' as usize]);
    assert_eq!(m.get_cursor(), (1, 0));
    // The glyph's 0 bits are the background.
    let bg = m.pixel_buffer()[0];
    assert_eq!(bg, 0x1, "top left pixel of 'A' is off");
    // Latin-1 and Windows-1252
    m.puts(&[0xE9, 0x9C, 0x80]);
    assert_eq!(cell_bits(&m, 1, 0, 0xC), DEFAULT_FONT[0xE9]);
    assert_eq!(cell_bits(&m, 3, 0, 0xC), DEFAULT_FONT[0x80]);
    assert_ne!(DEFAULT_FONT[0x9C], [0; 8], "œ has a glyph");
    for blank in [0x7F, 0x81, 0x8D, 0x8F, 0x90, 0x9D] {
        assert_eq!(DEFAULT_FONT[blank], [0; 8], "{blank:02X} is blank");
    }
}

#[test]
fn deferred_wrap() {
    let mut m = fresh();
    m.puts(&[b'x'; 32]);
    assert_eq!(m.cursor(), (31, 0, true), "the cursor waits on column 31");
    assert_eq!(m.get_cursor(), (32, 0), "GET_CURSOR says 32");
    assert_eq!(char_at(&m, 0, 1), b' ', "nothing on the next row yet");
    m.putc(b'y');
    assert_eq!(char_at(&m, 0, 1), b'y', "the next character wraps first");
    assert_eq!(m.get_cursor(), (1, 1));

    // '\n' after a full row moves down one row only.
    let mut m = fresh();
    m.puts(&[b'x'; 32]);
    m.putc(b'\n');
    assert_eq!(m.get_cursor(), (0, 1));
    // '\r' cancels it too, on the same row.
    let mut m = fresh();
    m.puts(&[b'x'; 32]);
    m.putc(b'\r');
    assert_eq!(m.get_cursor(), (0, 0));
    m.putc(b'y');
    assert_eq!(row_text(&m, 0), format!("y{}", "x".repeat(31)));
    // CURSOR cancels it.
    let mut m = fresh();
    m.puts(&[b'x'; 32]);
    m.set_cursor(3, 3);
    assert_eq!(m.cursor(), (3, 3, false));
}

#[test]
fn the_bottom_right_cell_does_not_scroll() {
    let mut m = fresh();
    m.set_cursor(0, 0);
    m.putc(b'T');
    m.set_cursor(31, 23);
    m.putc(b'Z');
    assert_eq!(char_at(&m, 31, 23), b'Z');
    assert_eq!(char_at(&m, 0, 0), b'T', "no scroll yet");
    assert_eq!(m.cursor(), (31, 23, true));
    m.putc(b'!');
    assert_eq!(char_at(&m, 31, 22), b'Z', "the screen moved up a row");
    assert_eq!(char_at(&m, 0, 23), b'!');
    assert_eq!(char_at(&m, 0, 0), b' ', "'T' left the screen");
    assert_eq!(m.get_cursor(), (1, 23));
}

#[test]
fn control_codes() {
    // '\b': back one cell, up a row from column 0, nothing at 0, 0.
    let mut m = fresh();
    m.putc(0x08);
    assert_eq!(m.get_cursor(), (0, 0));
    m.set_cursor(0, 5);
    m.putc(0x08);
    assert_eq!(m.get_cursor(), (31, 4));
    m.set_cursor(5, 5);
    m.putc(0x08);
    assert_eq!(m.get_cursor(), (4, 5));
    // '\b' with a pending wrap: back on the character just written.
    let mut m = fresh();
    m.puts(&[b'x'; 32]);
    m.puts(&[0x08, b' ', 0x08]);
    assert_eq!(char_at(&m, 31, 0), b' ', "erased");
    assert_eq!(m.cursor(), (31, 0, false));
    assert_eq!(char_at(&m, 0, 1), b' ', "nothing wrapped");
    // '\t': to the next multiple of 8; from column 24 on, to the next row.
    let mut m = fresh();
    m.putc(b'\t');
    assert_eq!(m.get_cursor(), (8, 0));
    m.set_cursor(9, 0);
    m.putc(b'\t');
    assert_eq!(m.get_cursor(), (16, 0));
    m.set_cursor(24, 0);
    m.putc(b'\t');
    assert_eq!(m.get_cursor(), (0, 1));
    m.puts(&[b'x'; 32]);
    m.putc(b'\t');
    assert_eq!(m.get_cursor(), (0, 2), "pending wrap: next row");
    assert_eq!(char_at(&m, 8, 0), b' ', "tab draws nothing");
    // '\n' on the last row scrolls.
    let mut m = fresh();
    m.putc(b'A');
    m.set_cursor(4, 23);
    m.putc(b'\n');
    assert_eq!(m.get_cursor(), (0, 23));
    assert_eq!(char_at(&m, 0, 0), b' ', "'A' scrolled away");
    // Other control codes: nothing at all.
    let mut m = fresh();
    for c in (0..0x20).filter(|c| ![0x08, 0x09, 0x0A, 0x0D].contains(c)) {
        m.putc(c);
    }
    assert_eq!(m.get_cursor(), (0, 0));
    assert!(m.text_buffer().iter().all(|c| c.code == b' '));
}

#[test]
fn cursor_color_clear() {
    let mut m = fresh();
    m.set_cursor(32, 0);
    m.set_cursor(0, 24);
    assert_eq!(m.get_cursor(), (0, 0), "off the grid: ignored");
    m.set_colors(0x1F, 0x23);
    assert_eq!(m.colors(), (0xF, 0x3), "low 4 bits");
    m.set_cursor(5, 5);
    m.putc(b'q');
    m.clear(4);
    assert!(m.pixel_buffer().iter().all(|&p| p == 4));
    assert!(
        m.text_buffer()
            .iter()
            .all(|c| c.code == b' ' && c.colors == 0x4F)
    );
    assert_eq!(m.get_cursor(), (0, 0));
    assert_eq!(m.colors(), (0xF, 0x3), "CLEAR keeps COLOR");
}

#[test]
fn scroll() {
    let mut m = fresh();
    m.set_colors(15, 2);
    m.puts(b"row0");
    m.set_cursor(0, 1);
    m.puts(b"row1");
    m.pixel(200, 15, 9); // in text row 1
    m.set_cursor(7, 7);
    m.scroll(0);
    assert_eq!(row_text(&m, 0), "row0");
    m.scroll(1);
    assert_eq!(row_text(&m, 0), "row1");
    assert_eq!(m.get_pixel(200, 7), 9, "graphics move too");
    assert_eq!(
        m.get_char(0, 23),
        (b' ', 0x2F),
        "new row in the text colours"
    );
    assert!(
        m.pixel_buffer()[23 * CELL * WIDTH..]
            .iter()
            .all(|&p| p == 2)
    );
    assert_eq!(m.get_cursor(), (7, 7), "the cursor stays");
    m.scroll(30);
    assert!(
        m.pixel_buffer().iter().all(|&p| p == 2),
        "n >= 24: every row"
    );
}

#[test]
fn defchar_and_text_reset() {
    let mut m = fresh();
    m.putc(b'~');
    let frame = [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF];
    m.define_char(b'~', frame);
    m.define_char(0x0A, frame); // a control code: ignored
    assert_eq!(m.font()[0x0A], [0; 8]);
    assert_eq!(
        cell_bits(&m, 0, 0, 15),
        DEFAULT_FONT[b'~' as usize],
        "drawn characters don't change"
    );
    m.putc(b'~');
    assert_eq!(cell_bits(&m, 1, 0, 15), frame);
    m.define_char(0xFF, frame);
    assert_eq!(m.font()[0xFF], frame);
    m.show_cursor(true);
    m.set_cursor(3, 4);
    m.text_reset();
    assert_eq!(*m.font(), DEFAULT_FONT);
    assert_eq!(m.colors(), (DEFAULT_FG, DEFAULT_BG));
    assert_eq!(m.cursor(), (0, 0, false));
    assert!(!m.cursor_shown());
    assert!(m.pixel_buffer().iter().all(|&p| p == DEFAULT_BG));
    assert_eq!(m.get_char(0, 0), (b' ', 0x1E));
}

#[test]
fn reset_button_keeps_the_picture() {
    let mut m = fresh();
    m.define_char(b'A', [0xAA; 8]);
    m.set_colors(3, 4);
    m.puts(b"keep");
    m.show_cursor(true);
    m.set_vsync(true);
    m.reset_button();
    assert_eq!(row_text(&m, 0), "keep");
    assert_eq!(*m.font(), DEFAULT_FONT);
    assert_eq!(m.colors(), (DEFAULT_FG, DEFAULT_BG));
    assert_eq!(m.cursor(), (0, 0, false));
    assert!(!m.cursor_shown() && !m.vsync());
}

#[test]
fn get_char_off_the_grid() {
    let m = fresh();
    assert_eq!(m.get_char(32, 0), (0, 0));
    assert_eq!(m.get_char(0, 24), (0, 0));
}

#[test]
fn pixels_lines_rectangles() {
    let mut m = fresh();
    m.pixel(255, 191, 0x1C);
    assert_eq!(m.get_pixel(255, 191), 0xC, "low 4 bits");
    m.pixel(0, 192, 5);
    assert_eq!(m.get_pixel(0, 192), 0, "off screen: 0");
    assert!(m.pixel_buffer().iter().filter(|&&p| p == 5).count() == 0);

    // A line: both ends, every octant gives the same pixels both ways.
    let lines = [
        (0, 0, 99, 0),
        (0, 0, 0, 99),
        (10, 10, 50, 30),
        (10, 10, 30, 50),
        (50, 30, 10, 10),
        (200, 5, 180, 100),
        (7, 7, 7, 7),
    ];
    for &(x0, y0, x1, y1) in &lines {
        let mut a = fresh();
        a.line(x0, y0, x1, y1, 3);
        let mut b = fresh();
        b.line(x1, y1, x0, y0, 3);
        let count = a.pixel_buffer().iter().filter(|&&p| p == 3).count();
        let dx = (x1 as i32 - x0 as i32).unsigned_abs() as usize;
        let dy = (y1 as i32 - y0 as i32).unsigned_abs() as usize;
        assert_eq!(count, dx.max(dy) + 1, "one pixel per step, both ends");
        assert_eq!(a.get_pixel(x0, y0), 3);
        assert_eq!(a.get_pixel(x1, y1), 3);
        assert!(a.pixel_buffer() == b.pixel_buffer(), "the same both ways");
    }
    // Clipped below the screen.
    let mut a = fresh();
    a.line(0, 180, 0, 250, 3);
    assert_eq!(a.pixel_buffer().iter().filter(|&&p| p == 3).count(), 12);

    // RECT / FILL_RECT
    let mut a = fresh();
    a.rect(10, 10, 0, 5, 3);
    a.rect(10, 10, 5, 0, 3);
    a.fill_rect(10, 10, 0, 5, 3);
    assert!(
        a.pixel_buffer().iter().all(|&p| p == 0),
        "w or h 0: nothing"
    );
    a.rect(10, 10, 4, 3, 3);
    assert_eq!(a.pixel_buffer().iter().filter(|&&p| p == 3).count(), 10);
    assert_eq!(a.get_pixel(11, 11), 0, "inside stays");
    a.fill_rect(10, 10, 4, 3, 6);
    assert_eq!(a.pixel_buffer().iter().filter(|&&p| p == 6).count(), 12);
    let mut a = fresh();
    a.fill_rect(250, 190, 255, 255, 7);
    assert_eq!(a.pixel_buffer().iter().filter(|&&p| p == 7).count(), 6 * 2);
    a.rect(250, 100, 20, 3, 8);
    // Top and bottom rows 250-255, left edge; the right edge is off screen.
    assert_eq!(
        a.pixel_buffer().iter().filter(|&&p| p == 8).count(),
        6 + 6 + 1
    );
    // Graphics leave the text alone.
    let mut a = fresh();
    a.putc(b'K');
    a.fill_rect(0, 0, 255, 191, 9);
    assert_eq!(a.get_char(0, 0).0, b'K');
    assert_eq!(a.get_cursor(), (1, 0));
    let _ = (HEIGHT, TEXT_ROWS);
}

// ----- VSYNC through the machine

#[test]
fn frames_are_60_per_second_of_cpu_cycles() {
    assert_eq!(frame_start(0), 0);
    assert_eq!(frame_start(1), 33_334);
    assert_eq!(frame_start(2), 66_667);
    assert_eq!(frame_start(3), 100_000);
    assert_eq!(frame_start(FRAME_RATE), CPU_HZ, "60 frames = 1 s");
}

/// A program that counts the VSYNC events: the handler adds 1 to `COUNT`
/// and keeps the event's data (the frame count) in `LAST`.
const COUNT: u16 = 0x8000;
const LAST: u16 = 0x8001;
const HANDLER: u16 = 0x4000;

fn vsync_machine(main: &[Instruction]) -> Machine {
    let handler = [
        none(O::Psf),
        one(O::Psh, A),
        reg_ram(O::Lod, A, COUNT),
        one(O::Inc, A),
        reg_ram(O::Str, A, COUNT),
        reg(O::Mov, A, I),
        reg_ram(O::Str, A, LAST),
        one(O::Pop, A),
        none(O::Pof),
        none(O::Irt),
    ];
    let mut m = Image::new()
        .program(main)
        .code(HANDLER, &handler)
        .vector(20, HANDLER) // the native display's port
        .build(MachineConfig::default());
    m.finish_boot();
    m
}

fn sleeping() -> Vec<Instruction> {
    // sti, then hlt in a loop: each event wakes the CPU once.
    vec![none(O::Sti), none(O::Hlt), imm16(O::Jmp, 1)]
}

#[test]
fn vsync_events_count_the_frames() {
    let mut m = vsync_machine(&sleeping());
    assert_eq!(
        m.peripherals.monitor.as_ref().unwrap().display_port(),
        Some(20)
    );
    m.run(CPU_HZ);
    assert_eq!(ram(&m, COUNT), 0, "VSYNC off at power-on");
    m.peripherals.monitor.as_mut().unwrap().set_vsync(true);
    let start = m.frame();
    m.run(CPU_HZ); // one second
    let frames = m.frame() - start;
    assert_eq!(frames, 60);
    assert_eq!(ram(&m, COUNT), 60, "one event per frame");
    assert_eq!(ram(&m, LAST), m.frame() as u8, "the count modulo 256");
    assert_eq!(m.board.iod.port(20), m.frame() as u8, "the port holds it");
    m.peripherals.monitor.as_mut().unwrap().set_vsync(false);
    m.run(CPU_HZ);
    assert_eq!(ram(&m, COUNT), 60, "off: no more events");
    assert_eq!(m.frame(), start + 120, "the count goes on");
}

#[test]
fn vsync_events_merge_while_interrupts_are_off() {
    // Interrupts off for 10 frames, then on.
    let main = vec![
        none(O::Cli),
        none(O::Nop),
        imm16(O::Jmp, 1), // a busy loop, interrupts off
    ];
    let mut m = vsync_machine(&main);
    m.peripherals.monitor.as_mut().unwrap().set_vsync(true);
    m.run(frame_start(m.frame() + 10) - m.ticks() + 10);
    let waiting: Vec<_> = m.board.iod.queue().copied().collect();
    let in_mailbox = m.iop.events;
    assert!(
        waiting.len() + in_mailbox as usize <= 2,
        "at most one in the queue (and one in the mailbox): {waiting:?}, {in_mailbox}"
    );
    if let Some(last) = waiting.last() {
        assert_eq!(last.data, m.frame() as u8, "the latest count");
    }
    m.reset(); // the reset button turns VSYNC off
    assert!(!m.peripherals.monitor.as_ref().unwrap().vsync());
}

#[test]
fn power_on_restarts_the_count_and_reset_keeps_it() {
    let mut m = vsync_machine(&sleeping());
    m.run(CPU_HZ / 2);
    let before = m.frame();
    assert!(before >= 29);
    m.reset();
    assert!(m.frame() >= before, "the reset button keeps the count");
    m.power_on();
    assert_eq!(m.frame(), 0);
}
