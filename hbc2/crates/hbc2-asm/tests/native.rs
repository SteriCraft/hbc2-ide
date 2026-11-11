//! The native commands through the mailbox (Peripherals/Native/Reference.md):
//! an assembled program streams command bytes and reads the replies, as the
//! library does. Each command must do what the `Monitor` method of the same
//! name does (hbc2-core/tests/native.rs tests those).

use std::path::Path;

use hbc2_asm::{MemoryFiles, Program, assemble};
use hbc2_core::iop::{LogEntry, command as cmd};
use hbc2_core::peripherals::Monitor;
use hbc2_core::{Machine, MachineConfig};

/// A program that sends `bytes` (interrupts off), then reads `replies`
/// bytes into `replies`, then stops.
fn program(bytes: &[u8], replies: usize) -> String {
    let data: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
    let mut s = String::from(
        "
_start:
    cli
    mov c, 1                ; STATUS
    mov d, 0                ; DATA
    mov x, cmds.msb
    mov y, cmds.lsb
    mov i, COUNT.msb
    mov j, COUNT.lsb
send:
    mov a, i
    or a, j
    jmz receive
    lod b, [x:y]
send_wait:
    in a, c
    and a, 2
    jmz send_wait
    out d, b
    add y, 1
    adc x, 0
    sub j, 1
    sbb i, 0
    jmp send
receive:
    mov x, replies.msb
    mov y, replies.lsb
    mov j, REPLIES
receive_next:
    cmp j, 0
    jme done
receive_wait:
    in a, c
    and a, 1
    jmz receive_wait
    in b, d
    str [x:y], b
    add y, 1
    adc x, 0
    dec j
    jmp receive_next
done:
    hlt
",
    );
    s = s
        .replace("COUNT.msb", &(bytes.len() >> 8).to_string())
        .replace("COUNT.lsb", &(bytes.len() & 0xFF).to_string())
        .replace("REPLIES", &replies.to_string());
    if data.is_empty() {
        s += ".data cmds 0\n";
    } else {
        for (k, chunk) in data.chunks(64).enumerate() {
            let name = if k == 0 {
                "cmds".to_string()
            } else {
                format!("cmds_{k}")
            };
            s += &format!(".data {name} {}\n", chunk.join(", "));
        }
    }
    s += &format!(".bss replies {}\n", replies.max(1));
    s
}

fn asm(source: &str) -> Program {
    assemble(
        Path::new("main.has"),
        &MemoryFiles::new().with("main.has", source),
    )
    .unwrap_or_else(|e| {
        panic!(
            "{}",
            e.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

/// Sends `bytes`, reads `replies` bytes; returns the machine and the replies.
fn send(bytes: &[u8], replies: usize, config: MachineConfig) -> (Machine, Vec<u8>) {
    let p = asm(&program(bytes, replies));
    let mut m = Machine::new(config, Some(&p.image)).unwrap();
    m.finish_boot();
    m.run(200 * bytes.len() as u64 + 2_000_000);
    assert!(m.cpu.is_halted(), "the program ends");
    let at = p.symbol("replies").unwrap().address as u16;
    let got = (0..replies as u16)
        .map(|k| m.board.ram.read(at + k))
        .collect();
    (m, got)
}

fn monitor(m: &Machine) -> &Monitor {
    m.peripherals.monitor.as_ref().unwrap()
}

fn same_screen(a: &Monitor, b: &Monitor) {
    assert!(a.pixel_buffer() == b.pixel_buffer(), "the same pixels");
    assert!(a.text_buffer() == b.text_buffer(), "the same cells");
    assert!(a.font() == b.font(), "the same font");
    assert_eq!(a.cursor(), b.cursor());
    assert_eq!(a.colors(), b.colors());
    assert_eq!(a.cursor_shown(), b.cursor_shown());
    assert_eq!(a.vsync(), b.vsync());
}

#[test]
fn every_command_does_what_the_monitor_does() {
    let frame = [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF];
    let mut bytes = vec![cmd::CLEAR, 0x02, cmd::COLOR, 0x0F, 0x04, cmd::PUTC, b'A'];
    bytes.extend([cmd::PUTS, 5, b'H', b'\n', 0xE9, b'\t', b'x']);
    bytes.extend([cmd::CURSOR, 30, 23, cmd::PUTS, 4, b'a', b'b', b'c', b'd']); // wraps, scrolls
    bytes.extend([cmd::DEFCHAR, b'~']);
    bytes.extend(frame);
    bytes.extend([cmd::DEFCHAR, 0x05, 1, 2, 3, 4, 5, 6, 7, 8]); // ignored, read whole
    bytes.extend([cmd::PUTC, b'~', cmd::SCROLL, 2, cmd::SHOW_CURSOR, 1]);
    bytes.extend([cmd::PIXEL, 255, 191, 0x0C, cmd::PIXEL, 3, 200, 0x0C]);
    bytes.extend([
        cmd::LINE,
        0,
        100,
        99,
        120,
        0x0E,
        cmd::RECT,
        20,
        20,
        30,
        10,
        0x0A,
    ]);
    bytes.extend([cmd::FILL_RECT, 250, 186, 255, 255, 0x09, cmd::VSYNC, 1]);
    bytes.extend([cmd::NOP, 0x1A, 0x40, 0x50, 0x70, 0x80, 0xFF]); // ignored, 1 byte each
    bytes.extend([cmd::PUTC, b'z']);
    let (m, _) = send(&bytes, 0, MachineConfig::default());

    let mut want = Monitor::new();
    want.clear(0x02);
    want.set_colors(0x0F, 0x04);
    want.putc(b'A');
    want.puts(&[b'H', b'\n', 0xE9, b'\t', b'x']);
    want.set_cursor(30, 23);
    want.puts(b"abcd");
    want.define_char(b'~', frame);
    want.putc(b'~');
    want.scroll(2);
    want.show_cursor(true);
    want.pixel(255, 191, 0x0C);
    want.pixel(3, 200, 0x0C);
    want.line(0, 100, 99, 120, 0x0E);
    want.rect(20, 20, 30, 10, 0x0A);
    want.fill_rect(250, 186, 255, 255, 0x09);
    want.set_vsync(true);
    want.putc(b'z');
    same_screen(monitor(&m), &want);
    assert_eq!(m.iop.partial_command(), &[] as &[u8], "nothing half read");

    // The debugger's log: PUTS with its length.
    assert!(m.iop.log.iter().any(|e| matches!(
        e,
        LogEntry::Native {
            opcode: cmd::PUTS,
            length: 6,
            args: [5, b'H', b'\n', 0xE9, b'\t', b'x'],
            ..
        }
    )));
    assert!(m.iop.log.contains(&LogEntry::Nop(0x40)));

    // TEXT_RESET, then VSYNC off.
    let (m, _) = send(
        &[
            cmd::PUTC,
            b'Q',
            cmd::VSYNC,
            1,
            cmd::TEXT_RESET,
            cmd::VSYNC,
            0,
        ],
        0,
        MachineConfig::default(),
    );
    let mut want = Monitor::new();
    want.text_reset();
    same_screen(monitor(&m), &want);
}

#[test]
fn the_reads_reply_in_order() {
    let bytes = [
        cmd::COLOR,
        0x0B,
        0x05,
        cmd::PUTS,
        32,
        b'0',
        b'1',
        b'2',
        b'3',
        b'4',
        b'5',
        b'6',
        b'7',
        b'8',
        b'9',
        b'0',
        b'1',
        b'2',
        b'3',
        b'4',
        b'5',
        b'6',
        b'7',
        b'8',
        b'9',
        b'0',
        b'1',
        b'2',
        b'3',
        b'4',
        b'5',
        b'6',
        b'7',
        b'8',
        b'9',
        b'A',
        b'B',
        cmd::GET_CURSOR, // 32, 0: a pending wrap
        cmd::GET_CHAR,
        31,
        0, // 'B', 0x5B
        cmd::GET_CHAR,
        32,
        0, // off the grid: 0, 0
        cmd::PIXEL,
        9,
        9,
        0x07,
        cmd::GET_PIXEL,
        9,
        9, // 7
        cmd::GET_PIXEL,
        9,
        192,           // off screen: 0
        cmd::GET_TIME, // the RTC: 2000-01-01 00:00:00
        cmd::IN,
        6, // still in step: the RTC's YEAR port
    ];
    let (m, replies) = send(&bytes, 2 + 2 + 2 + 1 + 1 + 6 + 1, MachineConfig::default());
    assert_eq!(
        replies,
        [
            32,
            0,
            b'B',
            0x5B,
            0,
            0,
            7,
            0,
            100,
            1,
            1,
            0,
            0,
            0,
            m.board.iod.port(6)
        ]
    );
}

#[test]
fn puts_of_0_and_255_characters() {
    let mut bytes = vec![cmd::PUTS, 0, cmd::PUTS, 255];
    bytes.extend((0..255).map(|k| b'a' + (k % 26) as u8));
    bytes.extend([cmd::GET_CURSOR]);
    let (m, replies) = send(&bytes, 2, MachineConfig::default());
    // 255 = 7 full rows (224) + 31: row 7, column 31.
    assert_eq!(replies, [31, 7]);
    assert_eq!(monitor(&m).get_char(30, 7).0, b'a' + (254 % 26) as u8);
}

#[test]
fn without_a_monitor_commands_are_read_whole() {
    let config = MachineConfig {
        monitor: false,
        ..MachineConfig::default()
    };
    let mut bytes = vec![cmd::DEFCHAR, b'A', 1, 2, 3, 4, 5, 6, 7, 8];
    bytes.extend([cmd::PUTS, 3, cmd::INFO, cmd::INFO, cmd::INFO]); // INFO bytes as text
    bytes.extend([cmd::GET_CHAR, 0, 0, cmd::GET_CURSOR, cmd::GET_PIXEL, 0, 0]);
    bytes.extend([cmd::INFO]);
    let (_, replies) = send(&bytes, 2 + 2 + 1 + 2, config);
    assert_eq!(&replies[..5], [0, 0, 0, 0, 0], "reads reply zeros");
    assert_eq!(
        &replies[5..],
        [2, 3],
        "then INFO: version 2, 3 devices (no display)"
    );
}

#[test]
fn get_time_without_a_clock_is_zeros() {
    let config = MachineConfig {
        rtc: false,
        ..MachineConfig::default()
    };
    let (_, replies) = send(&[cmd::GET_TIME], 6, config);
    assert_eq!(replies, [0; 6]);
}

#[test]
fn the_old_monitor_ports_reach_the_same_picture() {
    // OUT to the monitor's ports 1-5: text WRITE at 31, 23 and at 32, 0
    // (ignored), READ back, then pixel mode WRITE / READ.
    let o = |p: u8, v: u8| [cmd::OUT, p, v];
    let mut bytes = Vec::new();
    for (p, v) in [
        (1, 0x4E),
        (2, b'W'),
        (3, 31),
        (4, 23),
        (5, 1),
        (3, 32),
        (4, 0),
        (5, 1),
    ] {
        bytes.extend(o(p, v));
    }
    bytes.extend(o(3, 31));
    bytes.extend(o(4, 23));
    bytes.extend(o(5, 2)); // READ
    bytes.extend([cmd::IN, 1, cmd::IN, 2]);
    bytes.extend([cmd::GET_CHAR, 31, 23]);
    for (p, v) in [(5, 3), (1, 0x0D), (3, 200), (4, 100), (5, 1)] {
        bytes.extend(o(p, v));
    }
    bytes.extend([cmd::GET_PIXEL, 200, 100, cmd::GET_CURSOR]);
    let (m, replies) = send(&bytes, 2 + 2 + 1 + 2, MachineConfig::default());
    assert_eq!(replies, [0x4E, b'W', b'W', 0x4E, 0x0D, 0, 0]);
    let screen = monitor(&m);
    assert_eq!(screen.get_char(0, 0).0, b' ', "column 32 was ignored");
    assert_eq!(screen.cursor(), (0, 0, false), "the cursor didn't move");
}
