use std::path::Path;

use hbc2_asm::encode::{Kind, forms};
use hbc2_asm::{
    MemoryFiles, Options, Program, SymbolKind, Target, assemble, assemble_with, disassemble,
};
use hbc2_core::isa::{Instruction, STACK_RESET, encode};
use hbc2_core::{Flag, Machine, MachineConfig, Opcode, Register};

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

/// No peripherals, so no plug-in interrupts.
fn bare() -> MachineConfig {
    MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    }
}

#[test]
fn minimal_program() {
    let p = asm("_start:\n    mov a, 0x2A\n    jmp _start\n");
    assert_eq!(word(&p, 0x0000), encode::reg_imm8(O::Mov, A, 0x2A));
    assert_eq!(word(&p, 0x0004), encode::imm16(O::Jmp, 0x0000));
    assert_eq!(p.bytes_used, 8);
    assert_eq!(p.source_of(0x0004).unwrap().line, 3);
    assert_eq!(p.address_of(0, 2), Some(0x0000));
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
        encode::ramreg_offset(O::Inc, C, D, A, 0),
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
        assert_eq!(word(&p, n as u32 * 4), *e, "instruction {n}");
    }
}

#[test]
fn offset_and_stack_forms() {
    let p = asm("
.define FIELD 4
_start:
    lod a, [x:y + 3]
    str [x:y + FIELD], b
    cmp c, [x:y + 0x10]
    sub d, [i:j + 255]
    inc [c:d + 1]
    dec [c:d]
    lod a, [sp + 2]
    str [sp], b
    add c, [SP + 1]
    cmp d, [sp + 0]
    asp 2
    ssp 300
    rsp [x:y]
    wsp [c:d]
    wsp $0xFE00
");
    let expected = [
        encode::ramreg_offset(O::Lod, X, Y, A, 3),
        encode::ramreg_offset(O::Str, X, Y, B, 4),
        encode::ramreg_offset(O::Cmp, X, Y, C, 0x10),
        encode::ramreg_offset(O::Sub, I, J, D, 255),
        encode::ramreg_offset(O::Inc, C, D, A, 1),
        encode::ramreg_offset(O::Dec, C, D, A, 0),
        encode::stack_offset(O::Lod, A, 2),
        encode::stack_offset(O::Str, B, 0),
        encode::stack_offset(O::Add, C, 1),
        encode::stack_offset(O::Cmp, D, 0),
        encode::imm8(O::Asp, 2),
        encode::imm16(O::Ssp, 300),
        encode::reg16(O::Rsp, X, Y),
        encode::reg16(O::Wsp, C, D),
        encode::imm16(O::Wsp, 0xFE00),
    ];
    for (n, e) in expected.iter().enumerate() {
        assert_eq!(word(&p, n as u32 * 4), *e, "instruction {n}");
    }

    let e = errors("_start:\n    jmp [x:y + 2]\n");
    assert!(
        e[0].starts_with("main.has:2:9:") && e[0].contains("no offset"),
        "{e:?}"
    );
    assert!(errors("_start:\n    rsp [x:y + 1]\n")[0].contains("no offset"));
    assert!(errors("_start:\n    mov a, [sp + 1]\n")[0].contains("invalid operands"));
    assert!(errors("_start:\n    asp 70000\n")[0].contains("16 bits"));
}

#[test]
fn offsets_can_be_label_bytes_and_characters() {
    let p = asm(".define FIELD 2
.data table $0x12F0 1, 2, 3
_start:
    lod a, [x:y + table.lsb]
    str [x:b + table.msb], c
    add d, [sp + table.lsb]
    inc [i:j + 'A']
    lod a, [x:y + FIELD]
");
    let expected = [
        encode::ramreg_offset(O::Lod, X, Y, A, 0xF0),
        encode::ramreg_offset(O::Str, X, B, C, 0x12),
        encode::stack_offset(O::Add, D, 0xF0),
        encode::ramreg_offset(O::Inc, I, J, A, b'A'),
        encode::ramreg_offset(O::Lod, X, Y, A, 2),
    ];
    for (n, e) in expected.iter().enumerate() {
        assert_eq!(word(&p, n as u32 * 4), *e, "instruction {n}");
    }

    // Indexing a table: [x:b + table.lsb] with x = table.msb is table + b,
    // the carry into the high byte included.
    let p = asm(".data table $0x12F0 0
_start:
    mov x, table.msb
    mov b, 0x15
    mov a, 0x5A
    str [x:b + table.lsb], a
    hlt
");
    let mut m = Machine::new(bare(), Some(&p.image)).unwrap();
    m.run(100);
    assert_eq!(m.board.ram.read(0x1305), 0x5A);

    assert!(
        errors(
            "_start:
    lod a, [x:y + nowhere.lsb]
"
        )[0]
        .contains("unknown label")
    );
    assert!(
        errors(
            "_start:
    lod a, [x:y + -1]
"
        )[0]
        .contains("expected an offset")
    );
    assert!(
        errors(
            "_start:
    jmp [x:y + next.lsb]
next:
    nop
"
        )[0]
        .contains("no offset")
    );
}

#[test]
fn a_function_with_arguments_and_locals_runs() {
    // sum3(a, b, c) on the stack, result in a; one local; restores b.
    let p = asm("
_start:
    cli
    mov a, 10
    psh a
    mov a, 20
    psh a
    mov a, 30
    psh a
    cal sum3
    asp 3
    hlt

; Stack inside sum3, after psh b and ssp 1:
;   [sp] local, [sp + 1] saved b, [sp + 2] return MSB, [sp + 3] return LSB,
;   [sp + 4] = 30 (last pushed), [sp + 5] = 20, [sp + 6] = 10
sum3:
    psh b
    ssp 1
    lod a, [sp + 6]
    add a, [sp + 5]
    str [sp], a
    lod b, [sp + 4]
    lod a, [sp]
    add a, b
    asp 1
    pop b
    ret
");
    let mut m = Machine::new(bare(), Some(&p.image)).unwrap();
    m.run(100);
    assert!(m.cpu.is_halted());
    assert_eq!(m.cpu.reg(A), 60);
    assert_eq!(m.cpu.sp, STACK_RESET, "stack balanced");
}

#[test]
fn old_pointer_syntax_still_assembles() {
    // inc [x:y] was Reg16 (mode 0x5); it is now mode 0x4 with offset 0.
    let p = asm("
_start:
    mov x, 0x90
    mov y, 0x00
    inc [x:y]
    dec [x:y]
    dec [x:y]
    hlt
");
    assert_eq!(
        word(&p, 0x0008).mode(),
        Some(hbc2_core::AddrMode::RamRegImmReg)
    );
    let mut m = Machine::new(bare(), Some(&p.image)).unwrap();
    m.run(100);
    assert_eq!(m.board.ram.read(0x9000), 0xFF);
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
    assert_eq!(word(&p, 0x0000).v1(), 0xFF);
    assert_eq!(word(&p, 0x0004).v1(), 0xA5);
    assert_eq!(word(&p, 0x0008).v1(), (msg >> 8) as u8);
    assert_eq!(word(&p, 0x000C).v1(), msg as u8);
    assert_eq!(word(&p, 0x0010).vx() as u32, msg + 2);
    assert_eq!(word(&p, 0x0014).vx(), 0x0014, "end - 4");
    assert_eq!(&p.image[msg as usize..msg as usize + 5], b"Hi!\n\0");
}

#[test]
fn case_insensitive_mnemonics_and_registers() {
    let p = asm("_start:\n    MOV A, B\n    Jmp _start\n");
    assert_eq!(word(&p, 0x0000), encode::reg(O::Mov, A, B));
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
        0x0008,
        "first free byte after _start's 8 bytes"
    );
    assert_eq!(addr(&p, "second"), 0x000B);
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
    assert_eq!(addr(&p, "_start"), 0x0000);
    assert_eq!(addr(&p, "next"), 0x0004);
    assert_eq!(addr(&p, "handler"), 0xF000);
    assert_eq!(addr(&p, "after_handler"), 0xF004);
    assert_eq!(
        addr(&p, "double"),
        0x0008,
        "floating block placed after _start"
    );
    assert_eq!(p.files.len(), 2);
}

#[test]
fn defined_in_another_file() {
    let files = MemoryFiles::new()
        .with(
            "main.has",
            ".include \"lib.has\"
_start:
    nop
shared:
    ret
",
        )
        .with(
            "lib.has",
            "
shared:
    ret
",
        );
    let e = build(files, Target::Ram).expect_err("should fail");
    assert!(
        e[0].contains("'shared' is already defined (lib.has, line 2)"),
        "{e:?}"
    );
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

/// Windows file names ignore letter case: a second spelling is the same file.
#[cfg(windows)]
#[test]
fn include_once_ignores_case_on_windows() {
    let files = MemoryFiles::new()
        .with(
            "main.has",
            ".include \"lib/b.has\"
.include \"Lib/B.HAS\"
_start:
    jmp _start
",
        )
        .with(
            "lib/b.has",
            ".data b_var 0
",
        );
    let p = build(files, Target::Ram).unwrap_or_else(|e| {
        panic!(
            "{}",
            e.join(
                "
"
            )
        )
    });
    assert_eq!(p.files.len(), 2);
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
    assert_eq!(word(&p, 0x0000), encode::reg_imm8(O::Mov, A, 5));
    assert_eq!(word(&p, 0x0004), encode::reg_ram(O::Str, A, 0x0100));
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
    assert!(errors("_start: $0x0400\n")[0].contains("always at $0x0000"));
    assert!(errors(".include \"missing.has\"\n_start:\n")[0].contains("cannot read"));
    assert!(errors("_start:\n    nop\n.data v $0x0000 1\n")[0].contains("overlaps"));
    assert!(errors("_start:\n    lod a, $0x10000\n")[0].contains("16 bits"));
}

#[test]
fn several_errors_are_reported_at_once() {
    let e = errors("_start:\n    foo\n    mov a, 999\n    bar a\n");
    assert_eq!(e.len(), 3, "{e:?}");
}

#[test]
fn every_form_round_trips_through_the_disassembler() {
    let sample = |kind: Kind, offset: bool| match kind {
        Kind::R => "c",
        Kind::P if offset => "[x:y + 7]",
        Kind::P => "[x:y]",
        Kind::S => "[sp + 3]",
        Kind::I => "0x5a",
        Kind::W => "300",
        Kind::M => "$0x1234",
    };
    for info in hbc2_core::isa::INSTRUCTION_TABLE.iter() {
        for form in forms(info.opcode) {
            let operands: Vec<&str> = form
                .kinds
                .iter()
                .map(|&k| sample(k, form.takes_offset()))
                .collect();
            let text = format!("{} {}", info.mnemonic, operands.join(", "));
            let p = asm(&format!("_start:\n    {text}\n"));
            let w = word(&p, 0x0000);
            assert_eq!(w.mode(), Some(form.mode), "{text}");
            let again = asm(&format!("_start:\n    {}\n", disassemble(w.0)));
            assert_eq!(word(&again, 0x0000), w, "{text} -> {}", disassemble(w.0));
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
        0x0004,
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
        &p.image[0xFE00..0xFE02],
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
fn template_handler_preserves_the_main_program_state() {
    let handler = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/interruptHandler.has"),
    )
    .unwrap();
    // Registers and flags are set up before interrupts are allowed in.
    let main = "
.include \"interruptHandler.has\"
_start:
    cli
    mov a, 0x55
    mov b, 0x66
    stc
    ste
    sti
loop:
    jmp loop
";
    let files = MemoryFiles::new()
        .with("main.has", main)
        .with("interruptHandler.has", handler);
    let p = build(files, Target::Ram).unwrap_or_else(|e| panic!("{}", e.join("\n")));

    let mut m = Machine::new(MachineConfig::default(), Some(&p.image)).unwrap();
    m.run(10_000);
    assert_eq!(
        m.board.iod.queue().len(),
        0,
        "all plug-in interrupts handled"
    );
    assert_eq!(m.board.ram.read(addr(&p, "keyboardPort") as u16), 13);
    assert_eq!(
        (m.cpu.reg(Register::A), m.cpu.reg(Register::B)),
        (0x55, 0x66)
    );
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Equal));
    assert!(m.cpu.flag(Flag::Interrupt));
    assert_eq!(m.cpu.sp, STACK_RESET, "stack balanced");

    // The RTC (ports 6-12) gets the default handler, so its ticks change nothing.
    assert_eq!(m.board.ram.read(addr(&p, "rtcPort") as u16), 6);
    let entry = 0xFE00 + 2 * 6;
    let handler = addr(&p, "_rtc_default_handler") as u16;
    assert_eq!(m.board.ram.read_u16(entry), handler);
    for _ in 0..3 {
        m.advance_time(1_000_000_000);
        m.run(1_000);
    }
    assert_eq!(m.cpu.sp, STACK_RESET, "RTC ticks leave the stack balanced");
    assert_eq!(
        (m.cpu.reg(Register::A), m.cpu.reg(Register::B)),
        (0x55, 0x66)
    );
    assert_eq!(m.board.iod.queue().len(), 0, "RTC ticks handled");
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

#[test]
fn register_lists_expand_and_restore() {
    let p = asm("
_start:
    cli
    mov a, 1
    mov i, 2
    mov y, 3
    psh a, i, y
    mov a, 0
    mov i, 0
    mov y, 0
    pop a, i, y
done:
    jmp done
");
    // One instruction per register, all on the same source line.
    assert_eq!(word(&p, 0x0010), encode::reg(O::Psh, A, A));
    assert_eq!(word(&p, 0x0018), encode::reg(O::Psh, Y, A));
    assert_eq!(word(&p, 0x0028), encode::reg(O::Pop, Y, A));
    assert_eq!(word(&p, 0x0030), encode::reg(O::Pop, A, A));
    assert_eq!(p.source_of(0x0018).unwrap().line, 7);
    assert_eq!(
        p.address_of(0, 11),
        Some(0x0028),
        "breakpoints stop at the first one"
    );

    let mut m = Machine::new(MachineConfig::default(), Some(&p.image)).unwrap();
    m.run(20);
    assert_eq!((m.cpu.reg(A), m.cpu.reg(I), m.cpu.reg(Y)), (1, 2, 3));
    assert_eq!(m.cpu.sp, STACK_RESET, "stack balanced");

    assert!(errors("_start:\n    psh a, b, a\n")[0].contains("2:15"));
}

#[test]
fn wide_data_is_big_endian() {
    let p = asm("
.data16 vector $0x0100 handler, handler + 4
.data32 big 100000, -2
_start:
    jmp _start
handler: $0x1234
    irt
");
    let image = |a: u32, n: usize| p.image[a as usize..a as usize + n].to_vec();
    assert_eq!(image(0x0100, 4), [0x12, 0x34, 0x12, 0x38]);
    assert_eq!(
        image(addr(&p, "big"), 8),
        [0x00, 0x01, 0x86, 0xA0, 0xFF, 0xFF, 0xFF, 0xFE]
    );
    assert_eq!(p.symbol("big").unwrap().size, 8);
    assert!(errors(".data16 v nowhere\n_start:\n    jmp _start\n")[0].contains("unknown"));
}

#[test]
fn bss_is_placed_together_and_cleared_by_a_startup_loop() {
    let p = asm("
.define BUF_SIZE 300
.bss counter 2
.data before 7
_start:
    mov x, __bss_start.msb      ; fill the .bss area with 0xAA, like crt0 clears it
    mov y, __bss_start.lsb
    mov a, 0xAA
fill:
    cmp x, __bss_end.msb
    jme high_same
    jmp store
high_same:
    cmp y, __bss_end.lsb
    jme done
store:
    str [x:y], a
    inc y
    jmc carry
    jmp fill
carry:
    inc x
    jmp fill
done:
    hlt
.bss buffer BUF_SIZE
.data after 9
");
    let (start, end) = (addr(&p, "__bss_start"), addr(&p, "__bss_end"));
    assert_eq!(
        addr(&p, "counter"),
        start,
        "first .bss variable at the start"
    );
    assert_eq!(addr(&p, "buffer"), start + 2, "in source order, together");
    assert_eq!(end, start + 302);
    assert_eq!(p.symbol("buffer").unwrap().size, 300);
    assert_eq!(p.symbol("buffer").unwrap().kind, SymbolKind::Data);
    assert!(start > addr(&p, "after"), "after code and data");
    assert_eq!(p.symbol_at(start).unwrap().name, "counter");
    assert!(
        p.image[start as usize..end as usize]
            .iter()
            .all(|&b| b == 0)
    );
    let used = p.bytes_used;

    let mut m = Machine::new(bare(), Some(&p.image)).unwrap();
    m.run(10_000);
    let ram = |a: u32| m.board.ram.read(a as u16);
    assert!((start..end).all(|a| ram(a) == 0xAA));
    assert_eq!(ram(end), 0, "nothing after the area");
    assert_eq!(ram(addr(&p, "after")), 9);

    let without = asm("
_start:
    hlt
.data v 1
");
    assert_eq!(
        addr(&without, "__bss_start"),
        addr(&without, "__bss_end"),
        "empty area when there is no .bss"
    );
    assert_eq!(
        used,
        15 * 4 + 2,
        "code + data only: .bss takes no image bytes"
    );
}

#[test]
fn bss_errors() {
    let e = |src: &str| {
        let all = errors(&format!("_start:\n    hlt\n{src}\n"));
        all[0].clone()
    };
    assert!(e(".bss v 0").contains("out of range"));
    assert!(e(".bss v 65536").contains("out of range"));
    assert!(e(".bss v $0x9000 4").contains("no address"));
    assert!(e(".bss v").contains("expects a name and a size"));
    assert!(e(".bss v 4, 5").contains("after the size"));
    assert!(e(".bss __bss_end 4").contains("defined by the assembler"));
    assert!(e("__bss_start:\n    nop").contains("defined by the assembler"));
    assert!(e(".data v 1\n.bss v 1").contains("already defined"));
    assert!(e(".bss v 65500").contains("not enough free memory"));
}

#[test]
fn data_addresses_mixed_with_bytes() {
    let p = asm("
.data table $0x1200 1, 2, 3, 4
.data ptrs $0x2000 table, table + 2, 'A', table.lsb, \"hi\", table - 1
_start:
    jmp _start
");
    assert_eq!(
        p.image[0x2000..0x200B],
        [
            0x12, 0x00, 0x12, 0x02, b'A', 0x00, b'h', b'i', 0x11, 0xFF, 0
        ]
    );
    assert_eq!(p.symbol("ptrs").unwrap().size, 10);
    assert!(errors(".data v nowhere\n_start:\n    jmp _start\n")[0].contains("unknown"));
    assert!(errors(".data v 1, mov\n_start:\n    jmp _start\n")[0].contains("instruction"));
}

// ----- Removal of what can't be reached (Options::strip)

fn stripped_build(source: &str) -> Result<Program, Vec<String>> {
    let files = MemoryFiles::new().with("main.has", source);
    let options = Options {
        target: Target::Ram,
        strip: true,
    };
    assemble_with(Path::new("main.has"), &files, options)
        .map_err(|errors| errors.iter().map(ToString::to_string).collect())
}

fn stripped(source: &str) -> Program {
    stripped_build(source).unwrap_or_else(|e| panic!("{}", e.join("\n")))
}

fn removed(p: &Program) -> Vec<&str> {
    p.removed.iter().map(|r| r.name.as_str()).collect()
}

#[test]
fn strip_removes_what_nothing_reaches() {
    let source = "
_start:
    cal used
    hlt              ; falls through: loop is kept
loop:
    jmp loop
dead:            ; after a jmp, nothing reaches it
    mov a, 1
    ret
used:
    lod a, counter
    ret
unused:
    lod a, table
    cal dead
    ret
.data counter 0
.data table 1, 2, 3
.bss buffer 64
.bss kept_buffer 2
.data ptr kept_buffer
.data vector $0x9000 handler.msb, handler.lsb
handler:
    irt
";
    let p = stripped(source);
    assert_eq!(
        removed(&p),
        ["dead", "unused", "table", "buffer", "kept_buffer", "ptr"]
    );
    assert_eq!(
        p.removed.iter().map(|r| r.size).collect::<Vec<_>>(),
        [8, 12, 3, 64, 2, 2]
    );
    assert_eq!(p.bytes_removed(), 91);
    assert!(p.symbol("dead").is_none() && p.symbol("table").is_none());
    // used moved up to where dead was
    assert_eq!(addr(&p, "loop"), 0x0008);
    assert_eq!(addr(&p, "used"), 0x000C);
    assert_eq!(word(&p, 0x0000), encode::imm16(O::Cal, 0x000C));
    // handler is reached only from data at a fixed address
    assert!(p.symbol("handler").is_some() && p.symbol("vector").is_some());
    // A variable used only from data that is kept
    let p = stripped(&source.replace("lod a, counter", "lod a, ptr"));
    assert_eq!(
        removed(&p),
        ["dead", "unused", "counter", "table", "buffer"]
    );
    assert_eq!(addr(&p, "kept_buffer"), addr(&p, "__bss_start"));
    assert_eq!(addr(&p, "__bss_end"), addr(&p, "__bss_start") + 2);

    // Without strip: nothing removed.
    let all = asm(source);
    assert!(all.removed.is_empty());
    assert!(all.symbol("dead").is_some());
    assert_eq!(all.bytes_used, p.bytes_used + 8 + 12 + 1 + 3);
}

#[test]
fn strip_keeps_fall_through() {
    let p = stripped(
        "
_start:
    cal first
    hlt
first:
second:          ; empty piece above: falls into this one
    cmp a, 0
    jmz first    ; conditional: falls into third
third:
    ret
fourth:
    ret
",
    );
    assert_eq!(removed(&p), ["fourth"]);
    assert_eq!(addr(&p, "first"), addr(&p, "second"));
}

#[test]
fn strip_keeps_fixed_blocks_whole_and_kept_names() {
    let p = stripped(
        "
_start:
    jmp _start
computed2:
    ret
helper:
    ret
fixed: $0x8000
    irt
fixed_tail:      ; same fixed block: kept
    ret
computed:        ; still in the fixed block
    ret
.keep computed2
.data msb_only 0
.data user _start, msb_only.msb
",
    );
    assert_eq!(removed(&p), ["helper", "msb_only", "user"]);
    assert_eq!(addr(&p, "fixed_tail"), 0x8004);
    assert!(p.symbol("computed").is_some() && p.symbol("computed2").is_some());
    // msb / lsb count as uses
    let p = stripped("_start:\n    mov a, v.msb\n    hlt\n.data v 1\n");
    assert!(p.removed.is_empty());
}

#[test]
fn strip_reports_undefined_names_in_removed_code() {
    let e = stripped_build("_start:\n    hlt\nunused:\n    cal nowhere\n.data v missing\n")
        .expect_err("should fail");
    assert_eq!(e.len(), 2, "{e:?}");
    assert!(e[0].contains("unknown label or variable 'nowhere'"));
    assert!(e[1].contains("'missing'"));
    let e = errors("_start:\n    hlt\n.keep nothing\n");
    assert!(e[0].contains(".keep: 'nothing' is not defined"), "{e:?}");
    assert!(errors("_start:\n    hlt\n.keep\n")[0].contains("expects a label"));
    assert!(errors("_start:\n    hlt\n.keep a b\n")[0].contains("after the name"));
}

// ----- Library names give way to the program's (.library)

#[test]
fn library_names_give_way_to_the_program() {
    let files = MemoryFiles::new()
        .with(
            "main.has",
            ".include \"lib.has\"
_start:
    cal put
    cal print
    lod a, buffer
    hlt
put:             ; replaces the library's put
    mov a, 1
    ret
.data buffer 7   ; replaces the library's .bss buffer
",
        )
        .with(
            "lib.has",
            ".library
.include \"lib/more.has\"
print:
    cal put      ; binds to the program's put
    ret
put:
    mov a, 2
    ret
.bss buffer 16
",
        )
        .with("lib/more.has", "spare:\n    ret\n.data counter 0\n");
    let options = Options {
        target: Target::Ram,
        strip: true,
    };
    let p = assemble_with(Path::new("main.has"), &files, options).unwrap();
    let put = addr(&p, "put");
    assert_eq!(p.symbol("put").unwrap().file, 0, "the program's put");
    assert_eq!(
        word(&p, addr(&p, "print")),
        encode::imm16(O::Cal, put as u16)
    );
    assert_eq!(p.symbol("buffer").unwrap().kind, SymbolKind::Data);
    assert_eq!(p.image[addr(&p, "buffer") as usize], 7);
    let names: Vec<_> = p.removed.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "put (library, replaced)",
            "buffer (library, replaced)",
            "spare",
            "counter"
        ]
    );

    // The program's definition first: the same.
    let files = files.with(
        "main.has",
        "_start:\n    cal put\n    hlt\nput:\n    ret\n.include \"lib.has\"\n",
    );
    let p = assemble_with(Path::new("main.has"), &files, options).unwrap();
    assert_eq!(p.symbol("put").unwrap().file, 0);

    // Without strip the library's definitions stay, under their hidden names.
    let p = build(files, Target::Ram).unwrap();
    assert!(p.symbol("put (library, replaced)").is_some());
}

#[test]
fn library_duplicates_and_scope() {
    let lib = |main: &str, lib: &str| {
        let files = MemoryFiles::new()
            .with("main.has", main)
            .with("lib.has", lib)
            .with("other.has", "dup:\n    ret\n");
        build(files, Target::Ram)
    };
    // two library definitions
    let e = lib(
        "_start:\n    hlt\n.include \"lib.has\"\n",
        ".library\ntwice:\n    ret\ntwice:\n    ret\n",
    )
    .unwrap_err();
    assert!(e[0].contains("'twice' is already defined"), "{e:?}");
    // two program definitions
    let e = lib("_start:\n    hlt\ntwice:\n    ret\ntwice:\n    ret\n", "").unwrap_err();
    assert!(e[0].contains("'twice' is already defined"), "{e:?}");
    // .library covers the rest of its file and its includes, not the
    // including file: other.has is included after lib.has by main.has.
    let e = lib(
        "_start:\n    hlt\ndup:\n    ret\n.include \"lib.has\"\n.include \"other.has\"\n",
        ".library\n",
    )
    .unwrap_err();
    assert!(e[0].contains("'dup' is already defined"), "{e:?}");
    // names before .library are program names
    let e = lib(
        "_start:\n    hlt\ntwice:\n    ret\n.include \"lib.has\"\n",
        "twice:\n    ret\n.library\n",
    )
    .unwrap_err();
    assert!(e[0].contains("'twice' is already defined"), "{e:?}");
    // a library piece falling into a replaced one keeps the library's code
    let p = lib(
        "_start:\n    cal first\n    hlt\nsecond:\n    ret\n.include \"lib.has\"\n",
        ".library\nfirst:\n    nop\nsecond:\n    ret\n",
    )
    .unwrap();
    assert_eq!(
        addr(&p, "second (library, replaced)"),
        addr(&p, "first") + 4
    );
    assert!(errors("_start:\n    hlt\n.library now\n")[0].contains("takes nothing"));
}
