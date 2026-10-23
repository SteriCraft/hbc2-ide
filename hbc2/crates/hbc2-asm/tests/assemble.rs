use std::path::Path;

use hbc2_asm::encode::{Kind, forms};
use hbc2_asm::{MemoryFiles, Program, SymbolKind, Target, assemble, disassemble};
use hbc2_core::isa::{Instruction, encode};
use hbc2_core::{Machine, MachineConfig, Opcode, Register};

fn build(files: MemoryFiles, target: Target) -> Result<Program, Vec<String>> {
    assemble(Path::new("main.has"), &files, target)
        .map_err(|errors| errors.iter().map(ToString::to_string).collect())
}

fn asm(source: &str) -> Program {
    build(MemoryFiles::new().with("main.has", source), Target::Ram)
        .unwrap_or_else(|e| panic!("{}", e.join("\n")))
}

fn errors(source: &str) -> Vec<String> {
    build(MemoryFiles::new().with("main.has", source), Target::Ram).expect_err("should fail")
}

fn word(p: &Program, address: u32) -> Instruction {
    let a = address as usize;
    Instruction(u32::from_be_bytes(p.image[a..a + 4].try_into().unwrap()))
}

fn addr(p: &Program, name: &str) -> u32 {
    p.symbol(name)
        .unwrap_or_else(|| panic!("no symbol {name}"))
        .address
}

use Opcode as O;
use Register::*;

#[test]
fn minimal_program() {
    let p = asm("_start:\n    mov a, 0x2A\n    jmp _start\n");
    assert_eq!(word(&p, 0x0300), encode::reg_imm8(O::Mov, A, 0x2A));
    assert_eq!(word(&p, 0x0304), encode::imm16(O::Jmp, 0x0300));
    assert_eq!(p.bytes_used, 8);
    assert_eq!(p.source_of(0x0304).unwrap().line, 3);
    assert_eq!(p.address_of(0, 2), Some(0x0300));
}

#[test]
fn operand_forms() {
    let p = asm("
_start:
    add a, b
    add a, 'A'
    add a, var
    cmp c, [x:y]
    lod d, [x:y]
    str [x:y], d
    str var, a
    inc [c:d]
    dec var
    not a
    jmp [x:y]
    in a, b
    out c, d
    int 0x10
    psh a
    ret
.data var 0
");
    let var = addr(&p, "var") as u16;
    let expected = [
        encode::reg(O::Add, A, B),
        encode::reg_imm8(O::Add, A, b'A'),
        encode::reg_ram(O::Add, A, var),
        encode::ramreg_immreg(O::Cmp, X, Y, C),
        encode::ramreg_immreg(O::Lod, X, Y, D),
        encode::ramreg_immreg(O::Str, X, Y, D),
        encode::reg_ram(O::Str, A, var),
        encode::reg16(O::Inc, C, D),
        encode::imm16(O::Dec, var),
        encode::reg(O::Not, A, A),
        encode::reg16(O::Jmp, X, Y),
        encode::reg(O::In, A, B),
        encode::reg(O::Out, C, D),
        encode::imm8(O::Int, 0x10),
        encode::reg(O::Psh, A, A),
        encode::none(O::Ret),
    ];
    for (n, e) in expected.iter().enumerate() {
        assert_eq!(word(&p, 0x0300 + n as u32 * 4), *e, "instruction {n}");
    }
}

#[test]
fn values_and_expressions() {
    let p = asm("
_start:
    mov a, -1
    mov b, 0b1010_0101
    mov x, msg.msb
    mov y, msg.LSB
    lod c, msg + 2
    jmp end - 4
end:
.data msg \"Hi!\\n\", 0
");
    let msg = addr(&p, "msg");
    assert_eq!(word(&p, 0x0300).v1(), 0xFF);
    assert_eq!(word(&p, 0x0304).v1(), 0xA5);
    assert_eq!(word(&p, 0x0308).v1(), (msg >> 8) as u8);
    assert_eq!(word(&p, 0x030C).v1(), msg as u8);
    assert_eq!(word(&p, 0x0310).vx() as u32, msg + 2);
    assert_eq!(word(&p, 0x0314).vx(), 0x0314, "end - 4");
    assert_eq!(&p.image[msg as usize..msg as usize + 5], b"Hi!\n\0");
}

#[test]
fn case_insensitive_mnemonics_and_registers() {
    let p = asm("_start:\n    MOV A, B\n    Jmp _start\n");
    assert_eq!(word(&p, 0x0300), encode::reg(O::Mov, A, B));
    // Names stay case-sensitive.
    assert!(errors("_start:\n    jmp _START\n")[0].contains("unknown label"));
}

#[test]
fn data_is_placed_after_code_and_fixed_data_where_asked() {
    let p = asm("
.data first 1, 2, 3
.data vector $0x0100 0xF0, 0x00
_start:
    nop
    nop
.data second \"ab\"
");
    assert_eq!(addr(&p, "vector"), 0x0100);
    assert_eq!(
        addr(&p, "first"),
        0x0308,
        "first free byte after _start's 8 bytes"
    );
    assert_eq!(addr(&p, "second"), 0x030B);
    assert_eq!(&p.image[0x0100..0x0102], [0xF0, 0x00]);
    assert_eq!(p.symbol("first").unwrap().kind, SymbolKind::Data);
}

#[test]
fn blocks_fall_through_and_float() {
    let files = MemoryFiles::new()
        .with(
            "main.has",
            ".include \"lib/math.has\"
_start:
    cal double
next:            ; continues _start's block
    hlt
handler: $0xF000
    irt
after_handler:   ; continues at 0xF004
    irt
",
        )
        .with("lib/math.has", "double:\n    add a, a\n    ret\n");
    let p = build(files, Target::Ram).unwrap();
    assert_eq!(addr(&p, "_start"), 0x0300);
    assert_eq!(addr(&p, "next"), 0x0304);
    assert_eq!(addr(&p, "handler"), 0xF000);
    assert_eq!(addr(&p, "after_handler"), 0xF004);
    assert_eq!(
        addr(&p, "double"),
        0x0308,
        "floating block placed after _start"
    );
    assert_eq!(p.files.len(), 2);
}

#[test]
fn include_once_and_relative_paths() {
    let files = MemoryFiles::new()
        .with(
            "main.has",
            ".include \"a.has\"\n.include \"lib/b.has\"\n_start:\n    jmp _start\n",
        )
        .with("a.has", ".include \"lib/b.has\"\n.include \"a.has\"\n")
        .with(
            "lib/b.has",
            ".include \"../a.has\"\n.define B_VALUE 3\n.data b_var 0\n",
        );
    let p = build(files, Target::Ram).unwrap();
    assert_eq!(p.files.len(), 3);
    assert!(p.symbol("b_var").is_some());
}

#[test]
fn defines() {
    let p = asm("
.define PORT CMD_PORT
.define CMD_PORT 5
.define VECTOR $0x0100
_start:
    mov a, PORT
    str VECTOR, a
");
    assert_eq!(word(&p, 0x0300), encode::reg_imm8(O::Mov, A, 5));
    assert_eq!(word(&p, 0x0304), encode::reg_ram(O::Str, A, 0x0100));
    assert!(errors(".define A_ 1\n.define A_ 2\n_start:\n nop")[0].contains("already defined"));
    assert!(errors(".define X1 X2\n.define X2 X1\n_start:\n mov a, X1")[0].contains("loop"));
    assert!(errors(".define mov 1\n_start:\n nop")[0].contains("reserved"));
}

#[test]
fn error_messages_have_locations() {
    let e = errors("_start:\n    jmp nowhere\n");
    assert_eq!(
        e,
        ["main.has:2:9: error: unknown label or variable 'nowhere'"]
    );

    let e = errors("_start:\n    mov a, $0x10\n");
    assert!(
        e[0].starts_with("main.has:2:5:") && e[0].contains("mov r, r | mov r, value"),
        "{e:?}"
    );

    assert!(errors("    nop\n_start:\n")[0].contains("no label above it"));
    assert!(errors("loop:\n    nop\n")[0].contains("no _start"));
    assert!(errors("_start:\n_start:\n")[0].contains("already defined"));
    assert!(errors("_start: $0x0400\n")[0].contains("always at $0x0300"));
    assert!(errors(".include \"missing.has\"\n_start:\n")[0].contains("cannot read"));
    assert!(errors("_start:\n    nop\n.data v $0x0300 1\n")[0].contains("overlaps"));
    assert!(errors("_start:\n    lod a, $0x10000\n")[0].contains("16 bits"));
}

#[test]
fn several_errors_are_reported_at_once() {
    let e = errors("_start:\n    foo\n    mov a, 999\n    bar a\n");
    assert_eq!(e.len(), 3, "{e:?}");
}

#[test]
fn every_form_round_trips_through_the_disassembler() {
    let sample = |kind: Kind| match kind {
        Kind::R => "c",
        Kind::P => "[x:y]",
        Kind::I => "0x5a",
        Kind::M => "$0x1234",
    };
    for info in hbc2_core::isa::INSTRUCTION_TABLE.iter() {
        for form in forms(info.opcode) {
            let operands: Vec<&str> = form.kinds.iter().map(|&k| sample(k)).collect();
            let text = format!("{} {}", info.mnemonic, operands.join(", "));
            let p = asm(&format!("_start:\n    {text}\n"));
            let w = word(&p, 0x0300);
            assert_eq!(w.mode(), Some(form.mode), "{text}");
            let again = asm(&format!("_start:\n    {}\n", disassemble(w.0)));
            assert_eq!(word(&again, 0x0300), w, "{text} -> {}", disassemble(w.0));
        }
    }
}

#[test]
fn eeprom_target() {
    let source = "
_start:
    nop
.data table $0x12345 1, 2, 3
.data auto 7
far: $0x20000
    nop
";
    let p = build(MemoryFiles::new().with("main.has", source), Target::Eeprom).unwrap();
    assert_eq!(p.image.len(), 0x10_0000);
    assert_eq!(&p.image[0x12345..0x12348], [1, 2, 3]);
    assert_eq!(
        addr(&p, "auto"),
        0x0304,
        "automatic placement stays in boot ranges"
    );
    let warnings: Vec<String> = p.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings.iter().all(|w| w.contains("not in a range copied")));

    assert!(errors(".data far $0x12345 1\n_start:\n")[0].contains("past the end of memory"));
}

#[test]
fn templates_assemble_and_detect_peripherals() {
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/main.has");
    let p = hbc2_asm::assemble_file(&templates, Target::Ram).unwrap_or_else(|e| {
        panic!(
            "{}",
            e.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert_eq!(addr(&p, "_iod_interrupt_handler"), 0xF000);
    assert_eq!(
        &p.image[0x0100..0x0102],
        [0xF0, 0x00],
        "vector built from .msb/.lsb"
    );

    let mut m = Machine::new(MachineConfig::default(), Some(&p.image)).unwrap();
    m.run(10_000);
    let ram = |name: &str| m.board.ram.read(addr(&p, name) as u16);
    assert_eq!(ram("monitorPort"), 1);
    assert_eq!(ram("rtcPort"), 6);
    assert_eq!(ram("keyboardPort"), 13);
    assert_eq!(ram("firstInterrupt"), 0);
}

#[test]
fn language_documentation_example_prints_hi() {
    let doc_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../Documentation/HBC-2 Assembly Language.md");
    let doc = std::fs::read_to_string(doc_path).unwrap();
    let example = doc.split("```").nth(1).expect("first code block");
    let p = asm(example);

    let mut m = Machine::new(MachineConfig::default(), Some(&p.image)).unwrap();
    m.run(1_000);
    let screen = m.peripherals.monitor.as_ref().unwrap().text_buffer();
    assert_eq!(
        (screen[0].ascii, screen[1].ascii, screen[2].ascii),
        (b'H', b'i', 0)
    );
    assert_eq!(screen[0].colors, 0x0F);
}
