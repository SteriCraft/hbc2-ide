//! Execution trace (hbc2_core::trace): the CSV the Logisim circuit is compared with.

use std::path::Path;

use hbc2_asm::{MemoryFiles, Program, Target, assemble};
use hbc2_core::isa::IVT_START;
use hbc2_core::{Machine, MachineConfig, TRACE_HEADER, TraceWriter};

fn bare() -> MachineConfig {
    MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    }
}

fn asm(source: &str) -> Program {
    assemble(
        Path::new("main.has"),
        &MemoryFiles::new().with("main.has", source),
        Target::Ram,
    )
    .unwrap_or_else(|e| panic!("{e:?}"))
}

fn label(p: &Program, name: &str) -> u16 {
    p.symbol(name).unwrap().address as u16
}

/// Traces like `hbc2 trace`: until line `max` or until H is set.
fn trace(machine: &mut Machine, max: u64) -> String {
    let mut writer = TraceWriter::new(Vec::new(), Some(max));
    writer.start(&machine.cpu);
    while !writer.is_full() && !machine.cpu.is_halted() {
        machine.step_with(&mut writer);
    }
    String::from_utf8(writer.finish().unwrap()).unwrap()
}

/// The lines after the header, split into columns.
fn rows(csv: &str) -> Vec<Vec<&str>> {
    csv.lines()
        .skip(1)
        .map(|l| l.split(',').collect())
        .collect()
}

/// mov, psh, out, a loop closed by cmp + jme, cal / str / ret, hlt: checked by
/// hand against the encoding and the manual (tests/trace/loop.csv).
#[test]
fn loop_program_matches_the_expected_trace() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/trace");
    let program = hbc2_asm::assemble_with(
        &dir.join("loop.has"),
        &hbc2_asm::FileSystem,
        hbc2_asm::Options::new(Target::Ram),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let mut machine = Machine::new(bare(), Some(&program.image)).unwrap();
    machine.finish_boot();
    let got = trace(&mut machine, 1000);
    let expected = std::fs::read_to_string(dir.join("loop.csv")).unwrap();
    assert_eq!(got, expected);
    assert!(machine.cpu.is_halted());
}

#[test]
fn format() {
    let p = asm("
_start:
    mov a, 0xAB
    psh a
    mov c, 0x10
    out c, a
    cal f
    lod b, $0x1234
    hlt
f:
    ret
");
    let mut machine = Machine::new(bare(), Some(&p.image)).unwrap();
    machine.finish_boot();
    let csv = trace(&mut machine, 1000);

    assert!(csv.is_ascii());
    assert!(!csv.contains('\r') && !csv.contains(' '));
    assert!(csv.ends_with('\n'));
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some(TRACE_HEADER));
    assert_eq!(TRACE_HEADER, "n,pc,ir,a,b,c,d,i,j,x,y,sp,flags,mem_w,io_w");
    assert_eq!(
        lines.next(),
        Some("0,0000,,00,00,00,00,00,00,00,00,FE00,04,,"),
        "line 0: the reset state"
    );

    let hex = |s: &str, width: usize| {
        s.len() == width
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    };
    let rows = rows(&csv);
    assert_eq!(rows.len(), 9, "line 0 + 8 instructions");
    for (k, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), 15, "line {k}: {row:?}");
        assert_eq!(row[0], k.to_string());
        assert!(hex(row[1], 4), "pc {row:?}");
        if k > 0 {
            let ir = row[2];
            assert!(ir.len() % 2 == 0 && (2..=8).contains(&ir.len()), "{row:?}");
            assert!(hex(ir, ir.len()), "ir {row:?}");
        }
        for register in &row[3..11] {
            assert!(hex(register, 2), "register {row:?}");
        }
        assert!(hex(row[11], 4), "sp {row:?}");
        assert!(hex(row[12], 2), "flags {row:?}");
        for write in row[13].split(';').filter(|w| !w.is_empty()) {
            let (address, value) = write.split_once('=').unwrap();
            assert!(hex(address, 4) && hex(value, 2), "mem_w {row:?}");
        }
        for write in row[14].split(';').filter(|w| !w.is_empty()) {
            let (port, value) = write.split_once('=').unwrap();
            assert!(hex(port, 2) && hex(value, 2), "io_w {row:?}");
        }
    }
    // Hex in upper case, with leading zeros.
    assert_eq!(rows[1][3], "AB");
    assert_eq!(rows[2][13], "FDFF=AB");
    assert_eq!(rows[4][14], "10=AB");
    // CAL pushes the return address (after lod: 4 bytes, hlt: 1), LSB first.
    let ret = format!("{:04X}", label(&p, "f") - 5);
    assert_eq!(
        rows[5][13],
        format!("FDFE={};FDFD={}", &ret[2..], &ret[..2])
    );
    // 1 to 4 bytes: lod b, address is 4 bytes, hlt 1.
    assert_eq!(rows[7][2].len(), 8);
    assert_eq!(rows[8][2].len(), 2);
}

/// A hardware interrupt gets its own INT line: pc = interrupted address,
/// mem_w = the pushes of I, PC LSB, PC MSB.
#[test]
fn hardware_interrupt_line() {
    let p = asm("
_start:
    mov i, 0x77
    nop
    nop
    hlt
handler:
    irt
");
    let mut image = p.image.clone();
    let handler = label(&p, "handler");
    let entry = (IVT_START + 7 * 2) as usize;
    image[entry..entry + 2].copy_from_slice(&handler.to_be_bytes());
    let mut machine = Machine::new(bare(), Some(&image)).unwrap();
    machine.finish_boot();
    machine.board.iod.set_port(7, 0xAB);
    machine.board.iod.trigger_interrupt(7);

    let csv = trace(&mut machine, 1000);
    let rows = rows(&csv);
    // The interrupt is seen after the first instruction (the IOD raises
    // INT at the end of the first cycle), then the 2-cycle handshake.
    assert_eq!(rows[1][1], "0000");
    assert_eq!(
        rows[2].join(","),
        "2,0003,INT,00,00,00,00,AB,00,00,00,FDFD,00,FDFF=77;FDFE=03;FDFD=00,"
    );
    // irt: back to 0003, I restored, interrupts on again.
    assert_eq!(rows[3][1], format!("{handler:04X}"));
    assert_eq!(rows[3][7], "77");
    assert_eq!(rows[3][11], "FE00");
    assert_eq!(rows[3][12], "04");
    assert_eq!(rows[4][1], "0003");
}

/// `int n`: a normal line for the instruction, then an INT line when the
/// handler is entered (right away with I set).
#[test]
fn software_interrupt_line() {
    let p = asm("
_start:
    mov i, 0x42
    int 5
    hlt
handler:
    irt
");
    let mut image = p.image.clone();
    let handler = label(&p, "handler");
    let entry = (IVT_START + 5 * 2) as usize;
    image[entry..entry + 2].copy_from_slice(&handler.to_be_bytes());
    let mut machine = Machine::new(bare(), Some(&image)).unwrap();
    machine.finish_boot();
    let csv = trace(&mut machine, 1000);
    let rows = rows(&csv);
    assert_eq!(rows[2][1], "0003");
    assert_eq!(rows[2][13], "", "int writes nothing itself");
    assert_eq!(
        rows[3].join(","),
        "3,0006,INT,00,00,00,00,42,00,00,00,FDFD,00,FDFF=42;FDFE=06;FDFD=00,"
    );
}

#[test]
fn invalid_opcode_is_a_nop_line() {
    let mut image = vec![0u8; 0x1_0000];
    image[0] = 0xFC; // opcode 0x3F: not an instruction
    image[1] = 0x38; // hlt
    let mut machine = Machine::new(bare(), Some(&image)).unwrap();
    machine.finish_boot();
    let csv = trace(&mut machine, 1000);
    let rows = rows(&csv);
    assert_eq!(
        rows[1].join(","),
        "1,0000,FC,00,00,00,00,00,00,00,00,FE00,04,,"
    );
    assert_eq!(rows[2][1], "0001");
}

#[test]
fn invalid_addressing_mode_stops_the_trace() {
    let mut image = vec![0u8; 0x1_0000];
    image[0] = 0x74; // mov with mode bits 0000: mov has no such mode
    image[1] = 0x00;
    let mut machine = Machine::new(bare(), Some(&image)).unwrap();
    machine.finish_boot();
    let csv = trace(&mut machine, 1000);
    let rows = rows(&csv);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1].join(","),
        "1,0000,7400,00,00,00,00,00,00,00,00,FE00,80,,"
    );
}

#[test]
fn limit_and_mid_run_start() {
    let p = asm("
_start:
    inc a
    jmp _start
");
    let mut machine = Machine::new(bare(), Some(&p.image)).unwrap();
    machine.finish_boot();
    machine.step();
    machine.step();
    machine.step();
    // Line 0 is the current state, not the reset state.
    let csv = trace(&mut machine, 4);
    let rows = rows(&csv);
    assert_eq!(rows.len(), 5, "lines 0 to 4");
    assert_eq!(
        rows[0].join(","),
        "0,0002,,02,00,00,00,00,00,00,00,FE00,04,,"
    );
    assert_eq!(rows[4][0], "4");
}
