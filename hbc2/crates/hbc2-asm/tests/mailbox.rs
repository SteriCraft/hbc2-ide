//! The mailbox and the I/O processor model (hbc2-core mailbox.rs, iop.rs)
//! against the old IOD: the same program, once with `in` / `out` on the
//! IOD and once with the compat protocol through the mailbox
//! (io-redesign-proposal.md 9.3), must leave the same screen, the same
//! RAM and receive the same interrupts with the same data.

use std::path::Path;

use hbc2_asm::{MemoryFiles, Program, Target, assemble};
use hbc2_core::isa::IVT_START;
use hbc2_core::peripherals::{monitor, rtc};
use hbc2_core::{IopConfig, Machine, MachineConfig, Register};

/// Ports with a logging handler: port 0 (plug-in), monitor 1-5, RTC 6-12,
/// keyboard 13-14.
const HANDLED: u8 = 15;
const LOG: u16 = 0x9000;

/// The direct I/O routines: `io_out` writes a to port b, `io_in` reads port b into d.
const DIRECT: &str = "
io_out:
    out b, a
    ret
io_in:
    in d, b
    ret
io_info:             ; (the IOD has no INFO; d as the mailbox version leaves it)
    mov d, 1
    ret
";

/// The same through the mailbox: OUT = 0x01 p v, IN = 0x02 p and a reply.
/// A command is sent with interrupts masked (9.3).
const MAILBOX: &str = "
io_out:
    psf
    cli
    psh c
    mov c, 0x01
    cal mbx_send
    mov c, b
    cal mbx_send
    mov c, a
    cal mbx_send
    pop c
    pof
    ret
io_in:
    psf
    cli
    psh c
    mov c, 0x02
    cal mbx_send
    mov c, b
    cal mbx_send
    cal mbx_recv
    pop c
    pof
    ret
; INFO: the 3 reply bytes to info0..info2, d = 1
io_info:
    psf
    cli
    psh c
    mov c, 0x03
    cal mbx_send
    cal mbx_recv
    str info0, d
    cal mbx_recv
    str info1, d
    cal mbx_recv
    str info2, d
    mov d, 1
    pop c
    pof
    ret
; c -> TX, once TX is free (status bit 1); mirrors 0x21 / 0x20 on purpose
mbx_send:
    psh j
mbx_send_w:
    mov j, 0x21
    in j, j
    and j, 0x02
    jmz mbx_send_w
    mov j, 0x20
    out j, c
    pop j
    ret
; d <- RX, once a byte waits (status bit 0)
mbx_recv:
    psh j
mbx_recv_w:
    mov j, 0x01
    in j, j
    and j, 0x01
    jmz mbx_recv_w
    mov j, 0x00
    in d, j
    pop j
    ret
";

fn program(io: &str) -> String {
    let mut s = format!(
        "
; every interrupt logs (port, I) at 0x9000 + 2n, n counted in evn
_start:
    sti
wait_plug:
    lod a, evn
    cmp a, 12           ; 3 devices: 6 events on port 0
    jme plugged
    jmp wait_plug
plugged:
    mov b, {cmd}
    mov a, {text}
    cal io_out
    mov x, 0            ; 20 characters on row 3 from column 2
chars:
    mov b, {d0}         ; colours
    mov a, x
    add a, 0x10
    cal io_out
    mov b, {d1}         ; character
    mov a, x
    add a, 'A'
    cal io_out
    mov b, {px}
    mov a, x
    add a, 2
    cal io_out
    mov b, {py}
    mov a, 3
    cal io_out
    mov b, {cmd}
    mov a, {write}
    cal io_out
    inc x
    cmp x, 20
    jme read
    jmp chars
read:
    mov b, {px}
    mov a, 4
    cal io_out
    mov b, {cmd}
    mov a, {read}
    cal io_out
    mov b, {d0}
    cal io_in
    str rd0, d
    mov b, {d1}
    cal io_in
    str rd1, d
    mov b, {rtc_cmd}
    mov a, {get}
    cal io_out
    mov b, {year}
    cal io_in
    str year, d
    mov b, {second}
    cal io_in
    str second, d
    mov b, 200          ; a free port: its latch
    cal io_in
    str free, d
    cal io_info
    str infok, d
keys:
    lod a, evn
    cmp a, 16           ; two key events
    jme done
    jmp keys
done:
    cli
    hlt

evc:                    ; a = port; flags and a pushed by the stub
    psh x
    psh y
    lod y, evn
    mov x, 0x90
    str [x:y], a
    inc y
    str [x:y], i
    inc y
    str evn, y
    pop y
    pop x
    pop a
    pof
    irt
",
        cmd = 1 + monitor::port::CMD,
        d0 = 1 + monitor::port::DATA_0,
        d1 = 1 + monitor::port::DATA_1,
        px = 1 + monitor::port::POS_X,
        py = 1 + monitor::port::POS_Y,
        text = monitor::command::SWITCH_TO_TEXT_MODE,
        write = monitor::command::WRITE,
        read = monitor::command::READ,
        rtc_cmd = 6 + rtc::port::CMD,
        get = rtc::command::GET_TIME,
        year = 6 + rtc::port::YEAR,
        second = 6 + rtc::port::SECOND,
    );
    for p in 0..HANDLED {
        s += &format!("ev{p:02X}:\n    psf\n    psh a\n    mov a, {p}\n    jmp evc\n");
    }
    s += io;
    for v in ["evn", "rd0", "rd1", "year", "second", "free", "infok", "info0", "info1", "info2"] {
        s += &format!(".data {v} 0\n");
    }
    for p in 0..HANDLED {
        s += &format!(".data16 v{p:02X} ${:#06X} ev{p:02X}\n", IVT_START + 2 * p as u16);
    }
    s
}

fn asm(source: &str) -> Program {
    assemble(Path::new("main.has"), &MemoryFiles::new().with("main.has", source), Target::Ram)
        .unwrap_or_else(|e| panic!("{}", e.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")))
}

fn config(mailbox: Option<IopConfig>) -> MachineConfig {
    MachineConfig { monitor: true, rtc: true, keyboard: true, eeprom: None, mailbox }
}

fn var(m: &Machine, p: &Program, name: &str) -> u8 {
    m.board.ram.read(p.symbol(name).unwrap_or_else(|| panic!("no {name}")).address as u16)
}

/// Runs until the PC is at `label` (at most `max` cycles); returns the cycles.
fn run_to(m: &mut Machine, p: &Program, label: &str, max: u64) -> u64 {
    let at = p.symbol(label).unwrap().address as u16;
    let start = m.ticks();
    while m.cpu.pc != at {
        assert!(m.ticks() - start < max, "{label} not reached in {max} cycles");
        m.step();
    }
    m.ticks() - start
}

struct Outcome {
    log: Vec<u8>,
    vars: Vec<u8>,
    text: Vec<monitor::TextCell>,
    registers: [u8; 8],
    cycles: u64,
}

fn run(io: &str, mailbox: Option<IopConfig>) -> (Outcome, Program) {
    let p = asm(&program(io));
    let mut m = Machine::new(config(mailbox), Some(&p.image)).unwrap();
    assert_eq!(m.has_mailbox(), mailbox.is_some());
    let mut cycles = run_to(&mut m, &p, "keys", 2_000_000);
    assert!(m.key_event(0x41, true) && m.key_event(0x41, false));
    cycles += run_to(&mut m, &p, "done", 200_000);
    m.step(); // cli
    m.step(); // hlt
    assert!(m.cpu.is_halted());
    let n = var(&m, &p, "evn") as u16;
    let out = Outcome {
        log: (0..n).map(|k| m.board.ram.read(LOG + k)).collect(),
        vars: ["evn", "rd0", "rd1", "year", "second", "free"].iter().map(|v| var(&m, &p, v)).collect(),
        text: m.peripherals.monitor.as_ref().unwrap().text_buffer().to_vec(),
        registers: std::array::from_fn(|r| m.cpu.reg(Register::ALL[r])),
        cycles,
    };
    (out, p)
}

#[test]
fn the_mailbox_runs_the_iod_protocol() {
    let (direct, _) = run(DIRECT, None);
    let (mbx, _) = run(MAILBOX, Some(IopConfig::default()));
    // Plug-in (port 0: first port, ID) x 3, then the key press (port 13) and release (14).
    assert_eq!(
        direct.log,
        [0, 1, 0, 0x4A, 0, 6, 0, 0x95, 0, 13, 0, 0xAA, 13, 0x41, 14, 0x41],
        "the old IOD's events"
    );
    assert_eq!(mbx.log, direct.log, "the same events, in order, with the same data in I");
    assert_eq!(mbx.vars, direct.vars, "evn, monitor READ, RTC year / second, a free latch");
    assert_eq!((direct.vars[1], direct.vars[2]), (0x12, b'C'), "READ at column 4");
    assert!(mbx.text == direct.text, "the same screen");
    assert_eq!(mbx.text[3 * monitor::TEXT_COLUMNS + 2 + 19].ascii, b'A' + 19);
    assert_eq!(mbx.registers, direct.registers);
    assert!(mbx.cycles > direct.cycles, "the protocol costs time: {} vs {}", mbx.cycles, direct.cycles);
}

#[test]
fn info_reports_the_protocol_and_the_devices() {
    let p = asm(&program(MAILBOX));
    let mut m = Machine::new(config(Some(IopConfig::default())), Some(&p.image)).unwrap();
    run_to(&mut m, &p, "keys", 2_000_000);
    assert_eq!(var(&m, &p, "infok"), 1);
    assert_eq!(
        [var(&m, &p, "info0"), var(&m, &p, "info1"), var(&m, &p, "info2")],
        [hbc2_core::iop::PROTOCOL_VERSION, hbc2_core::iop::FIRMWARE_VERSION, 0x07],
        "version 1, the emulator's firmware, monitor + RTC + keyboard"
    );
    assert_eq!(m.iop.partial_command(), &[] as &[u8]);
}

/// Breaking the firmware rule (I15): the second of two back-to-back events
/// is written during the CPU's clear and lost.
#[test]
fn breaking_the_rule_loses_events() {
    let p = asm(&program(MAILBOX));
    let eager = IopConfig { service_cycles: 1, rule_cycles: 0 };
    let mut m = Machine::new(config(Some(eager)), Some(&p.image)).unwrap();
    m.run(500_000);
    let n = var(&m, &p, "evn");
    assert!(n < 12, "with the rule broken, some of the 6 plug-in events are lost ({} logged)", n / 2);
    assert!(n >= 2, "the first one arrives");
    assert_eq!(m.iop.events, 6, "the MCU wrote all 6: the others were cleared by the CPU's evt_clr");
    assert!(m.cpu.pc < p.symbol("plugged").unwrap().address as u16, "still waiting for the devices");

    // With the rule (and the same fast MCU), all arrive.
    let fast = IopConfig { service_cycles: 1, rule_cycles: 2 };
    let mut m = Machine::new(config(Some(fast)), Some(&p.image)).unwrap();
    run_to(&mut m, &p, "keys", 2_000_000);
    assert_eq!(var(&m, &p, "evn"), 12);
}

/// A halted CPU with nothing to do is idle, so `run` doesn't simulate the
/// rest of its budget; a key press wakes the I/O processor up again.
#[test]
fn idle_with_a_mailbox() {
    let p = asm(&program(MAILBOX));
    let mut m = Machine::new(config(Some(IopConfig::default())), Some(&p.image)).unwrap();
    run_to(&mut m, &p, "keys", 2_000_000);
    m.key_event(0x41, true);
    m.key_event(0x41, false);
    run_to(&mut m, &p, "done", 200_000);
    m.run(10);
    assert!(m.cpu.is_halted() && m.is_idle());
    let before = m.ticks();
    m.run(1_000_000_000);
    assert_eq!(m.ticks() - before, 1_000_000_000);
    // Interrupts are off (cli): the event stays pending, INT high.
    m.key_event(0x42, true);
    assert!(!m.is_idle());
    m.run(100);
    assert!(m.board.mailbox.as_ref().unwrap().evt_pend && m.board.int);
    assert!(m.is_idle(), "stopped: the pending event changes nothing");
}
