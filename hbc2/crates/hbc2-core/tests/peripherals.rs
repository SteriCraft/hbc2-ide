//! Peripherals, driven by programs through the IOD.

mod common;

use common::*;
use hbc2_core::isa::Instruction;
use hbc2_core::isa::encode::*;
use hbc2_core::peripherals::{Eeprom, eeprom, monitor, rtc};
use hbc2_core::{Machine, MachineConfig};

/// Builds a program from instruction groups, ending in a halt loop
/// (HLT re-enables interrupts, so the CPU may wake up and must halt again).
fn program(parts: &[&[Instruction]]) -> Vec<Instruction> {
    let mut v = vec![none(O::Cli)];
    for part in parts {
        v.extend_from_slice(part);
    }
    let halt = at(v.len() as u16);
    v.push(none(O::Hlt));
    v.push(imm16(O::Jmp, halt));
    v
}

/// Port-0 handler (plug-in interrupts) that just returns.
const IGNORE_HANDLER: u16 = 0xFF00;

fn run(config: MachineConfig, parts: &[&[Instruction]]) -> Machine {
    let mut m = Image::new()
        .program(&program(parts))
        .vector(0, IGNORE_HANDLER)
        .code(IGNORE_HANDLER, &[none(O::Irt)])
        .build(config);
    m.run(10_000);
    assert!(m.cpu.is_halted(), "program should end on HLT");
    m
}

#[test]
fn ports_are_allocated_in_order_and_announced() {
    let m = Machine::new(
        MachineConfig {
            eeprom: Some(Eeprom::blank()),
            ..MachineConfig::default()
        },
        None,
    )
    .unwrap();
    let p = &m.peripherals;
    use hbc2_core::peripherals::Peripheral as _;
    assert_eq!(p.monitor.as_ref().unwrap().first_port(), Some(1));
    assert_eq!(p.rtc.as_ref().unwrap().first_port(), Some(6));
    assert_eq!(p.keyboard.as_ref().unwrap().first_port(), Some(13));
    assert_eq!(p.eeprom.as_ref().unwrap().first_port(), Some(15));
    let announced: Vec<(u8, u8)> = m.board.iod.queue().map(|i| (i.port, i.data)).collect();
    assert_eq!(
        announced,
        [
            (0, 1),
            (0, 0x4A),
            (0, 6),
            (0, 0x95),
            (0, 13),
            (0, 0xAA),
            (0, 15),
            (0, 0xE1)
        ]
    );
}

// ----- Monitor (ports 1-5 when plugged alone)

const MON: u8 = 1;

fn monitor_only() -> MachineConfig {
    MachineConfig {
        monitor: true,
        ..bare()
    }
}

#[test]
fn monitor_text_write() {
    let m = run(
        monitor_only(),
        &[
            &out(MON + monitor::port::DATA_0, 0x1F),
            &out(MON + monitor::port::DATA_1, b'A'),
            &out(MON + monitor::port::POS_X, 3),
            &out(MON + monitor::port::POS_Y, 2),
            &out(MON + monitor::port::CMD, monitor::command::WRITE),
        ],
    );
    let screen = m.peripherals.monitor.as_ref().unwrap();
    assert_eq!(screen.mode(), monitor::Mode::Text);
    let cell = screen.text_buffer()[3 + 2 * monitor::TEXT_COLUMNS];
    assert_eq!(
        (cell.background(), cell.foreground(), cell.ascii),
        (0x1, 0xF, b'A')
    );
}

#[test]
fn monitor_only_acts_when_cmd_is_written() {
    let m = run(
        monitor_only(),
        &[
            &out(MON + monitor::port::CMD, monitor::command::WRITE),
            &out(MON + monitor::port::DATA_1, b'Z'), // CMD not written again: no effect
        ],
    );
    let screen = m.peripherals.monitor.as_ref().unwrap();
    assert_eq!(screen.text_buffer()[0].ascii, 0);
}

#[test]
fn monitor_pixel_write_and_read() {
    let m = run(
        monitor_only(),
        &[
            &out(
                MON + monitor::port::CMD,
                monitor::command::SWITCH_TO_PIXEL_MODE,
            ),
            &out(MON + monitor::port::DATA_0, 0xAB),
            &out(MON + monitor::port::POS_X, 255),
            &out(MON + monitor::port::POS_Y, 191),
            &out(MON + monitor::port::CMD, monitor::command::WRITE),
            &out(MON + monitor::port::DATA_0, 0x00),
            &out(MON + monitor::port::CMD, monitor::command::READ),
            &input(A, MON + monitor::port::DATA_0),
        ],
    );
    let screen = m.peripherals.monitor.as_ref().unwrap();
    assert_eq!(screen.mode(), monitor::Mode::Pixel);
    assert_eq!(
        screen.pixel_buffer()[255 + 191 * monitor::WIDTH],
        0x0B,
        "4-bit colour"
    );
    assert_eq!(m.cpu.reg(A), 0x0B);
}

#[test]
fn monitor_invalid_positions() {
    let m = run(
        monitor_only(),
        &[
            &out(MON + monitor::port::DATA_1, b'X'),
            &out(MON + monitor::port::POS_X, 42), // one past the last column
            &out(MON + monitor::port::CMD, monitor::command::WRITE),
            &out(MON + monitor::port::CMD, monitor::command::READ),
            &input(A, MON + monitor::port::DATA_1),
        ],
    );
    let screen = m.peripherals.monitor.as_ref().unwrap();
    assert!(
        screen.text_buffer().iter().all(|c| c.ascii == 0),
        "write ignored"
    );
    assert_eq!(m.cpu.reg(A), 0, "READ of an invalid position returns 0x00");
}

// ----- Keyboard

#[test]
fn keyboard_press_and_release_use_two_ports() {
    let mut m = Image::new()
        .program(&[none(O::Cli), imm16(O::Jmp, at(1))])
        .build(MachineConfig {
            keyboard: true,
            ..bare()
        });
    m.run(10);
    assert!(m.key_event(0x10, true));
    assert!(m.key_event(0x10, false));
    let queue: Vec<(u8, u8)> = m
        .board
        .iod
        .queue()
        .skip(2)
        .map(|i| (i.port, i.data))
        .collect();
    assert_eq!(queue, [(1, 0x10), (2, 0x10)]);
}

#[test]
fn key_event_without_keyboard_is_dropped() {
    let mut m = machine(&[]);
    assert!(!m.key_event(0x10, true));
}

// ----- EEPROM (ports 1-5 when plugged alone)

const EE: u8 = 1;

fn eeprom_image() -> Vec<u8> {
    let mut image = vec![0u8; eeprom::MEMORY_SIZE];
    // Program at 0x0000: read EEPROM[0x12345] into A, then write 0x55 to EEPROM[0x01010].
    let code = program(&[
        &out(EE + eeprom::port::ADDR_0, 0x01),
        &out(EE + eeprom::port::ADDR_1, 0x23),
        &out(EE + eeprom::port::ADDR_2, 0x45),
        &out(EE + eeprom::port::CMD, eeprom::command::READ),
        &input(A, EE + eeprom::port::DATA),
        &out(EE + eeprom::port::ADDR_0, 0x00),
        &out(EE + eeprom::port::ADDR_1, 0x10),
        &out(EE + eeprom::port::ADDR_2, 0x10),
        &out(EE + eeprom::port::DATA, 0x55),
        &out(EE + eeprom::port::CMD, eeprom::command::WRITE),
    ]);
    for (n, i) in code.iter().enumerate() {
        image[n * 4..n * 4 + 4].copy_from_slice(&i.to_bytes());
    }
    image[0xFE00..0xFE02].copy_from_slice(&[0x01, 0xF0]); // IVT: port 0 -> 0x01F0
    image[0x01F0..0x01F4].copy_from_slice(&none(O::Irt).to_bytes()); // handler, first 512 bytes
    image[0xF000] = 0xEE; // the old handlers area: not copied any more
    image[0x0200] = 0xEE; // just after the first 512 bytes
    image[0x12345] = 0x99;
    image
}

fn eeprom_machine() -> Machine {
    let config = MachineConfig {
        eeprom: Some(Eeprom::from_image(eeprom_image()).unwrap()),
        ..bare()
    };
    // A RAM image is ignored when an EEPROM is plugged.
    Machine::new(config, Some(&vec![0x11; hbc2_core::isa::RAM_SIZE])).unwrap()
}

#[test]
fn eeprom_copies_boot_ranges_into_ram() {
    let m = eeprom_machine();
    assert_eq!(ram(&m, 0xFE01), 0xF0, "IVT copied");
    assert_eq!(
        ram(&m, 0x01F0),
        none(O::Irt).to_bytes()[0],
        "first 512 bytes copied"
    );
    assert_eq!(ram(&m, 0x0200), 0x00, "not copied");
    assert_eq!(ram(&m, 0xF000), 0x00, "handlers are not copied");
    assert_eq!(ram(&m, 0x8000), 0x00, "RAM image ignored");
}

#[test]
fn eeprom_read_and_write_from_a_program() {
    let mut m = eeprom_machine();
    m.run(10_000);
    assert!(m.cpu.is_halted());
    assert_eq!(m.cpu.reg(A), 0x99);
    assert_eq!(
        m.peripherals.eeprom.as_ref().unwrap().memory()[0x1010],
        0x55
    );

    m.power_on();
    assert_eq!(
        m.peripherals.eeprom.as_ref().unwrap().memory()[0x1010],
        0x55,
        "non-volatile"
    );
}

#[test]
fn eeprom_rejects_wrong_size() {
    assert!(Eeprom::from_image(vec![0; 1000]).is_err());
}

// ----- RTC (ports 1-7 when plugged alone)

const RTC: u8 = 1;

fn rtc_only() -> MachineConfig {
    MachineConfig {
        rtc: true,
        ..bare()
    }
}

#[test]
fn rtc_interrupts_every_second() {
    let mut m = Image::new()
        .program(&[none(O::Cli), imm16(O::Jmp, at(1))])
        .build(rtc_only());
    m.advance_time(500_000_000);
    assert_eq!(
        m.board.iod.queue().len(),
        2,
        "only the plug-in interrupts so far"
    );
    m.advance_time(600_000_000);
    let last = *m.board.iod.queue().last().unwrap();
    assert_eq!(last.port, RTC + rtc::port::YEAR);
    assert_eq!(m.board.iod.port(RTC + rtc::port::SECOND), 1);
    assert_eq!(m.board.iod.port(RTC + rtc::port::YEAR), 100, "2000 - 1900");
}

#[test]
fn rtc_set_and_get_time() {
    let m = run(
        rtc_only(),
        &[
            &out(RTC + rtc::port::YEAR, 124),
            &out(RTC + rtc::port::MONTH, 2),
            &out(RTC + rtc::port::DAY, 29),
            &out(RTC + rtc::port::HOUR, 23),
            &out(RTC + rtc::port::MINUTE, 59),
            &out(RTC + rtc::port::SECOND, 59),
            &out(RTC + rtc::port::CMD, rtc::command::SET_TIME),
            &input(A, RTC + rtc::port::YEAR), // 1 = success
            &out(RTC + rtc::port::MONTH, 13),
            &out(RTC + rtc::port::CMD, rtc::command::SET_TIME),
            &input(B, RTC + rtc::port::YEAR), // 0 = invalid
            &out(RTC + rtc::port::CMD, rtc::command::GET_TIME),
            &input(C, RTC + rtc::port::DAY),
            &input(D, RTC + rtc::port::CMD),
        ],
    );
    assert_eq!(m.cpu.reg(A), 1);
    assert_eq!(m.cpu.reg(B), 0);
    assert_eq!(m.cpu.reg(C), 29);
    assert_eq!(m.cpu.reg(D), rtc::command::NOP, "CMD reset after execution");

    let mut m = m;
    m.advance_time(1_000_000_000);
    let now = m.peripherals.rtc.as_ref().unwrap().now();
    assert_eq!(
        (
            now.year, now.month, now.day, now.hour, now.minute, now.second
        ),
        (2024, 3, 1, 0, 0, 0)
    );
}

#[test]
fn rtc_new_year() {
    let mut m = run(
        rtc_only(),
        &[
            &out(RTC + rtc::port::YEAR, 99),
            &out(RTC + rtc::port::MONTH, 12),
            &out(RTC + rtc::port::DAY, 31),
            &out(RTC + rtc::port::HOUR, 23),
            &out(RTC + rtc::port::MINUTE, 59),
            &out(RTC + rtc::port::SECOND, 59),
            &out(RTC + rtc::port::CMD, rtc::command::SET_TIME),
        ],
    );
    m.advance_time(1_000_000_000);
    let now = m.peripherals.rtc.as_ref().unwrap().now();
    assert_eq!((now.year, now.month, now.day), (2000, 1, 1));
}
