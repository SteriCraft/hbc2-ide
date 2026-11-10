//! The checks have teeth: each mutation of the microcode source must be
//! rejected by the generator's rules or fail a comparison with the emulator.

use hbc2_ucode::{SOURCE, check, generate};

fn caught(what: &str, find: &str, replace: &str) -> String {
    // Escapes, not literal line breaks: Git's CRLF conversion rewrites those.
    let source = SOURCE.replace("\r\n", "\n");
    assert!(source.contains(find), "{what}: `{find}` not in the source");
    let text = source.replacen(find, replace, 1);
    let mc = match generate::build(&text) {
        Err(e) => return format!("{what}: generator: {}", e[0]),
        Ok(mc) => mc,
    };
    if let Err(e) = check::boot(&mc) {
        return format!("{what}: boot: {}", e.lines().next().unwrap_or(""));
    }
    if let Err(e) = check::every_instruction_and_mode(&mc, 4) {
        return format!("{what}: instructions: {}", e.lines().nth(1).unwrap_or(""));
    }
    if let Err(e) = check::random_programs(&mc, 300) {
        return format!(
            "{what}: random programs: {}",
            e.lines().next().unwrap_or("")
        );
    }
    panic!("{what}: not caught");
}

#[test]
fn mutations_are_caught() {
    let cases = [
        ("STI without the delay", "I<-1, end nocheck", "I<-1, end"),
        (
            "JMx: taken and not taken swapped",
            "fv2, if jc taken else skip",
            "fv2, if jc skip else taken",
        ),
        (
            "interrupt entry leaves H",
            "lat<-sp-1, sp<-sp-1, H<-0\nIE3h",
            "lat<-sp-1, sp<-sp-1\nIE3h",
        ),
        ("no INR", "lat<-0, inr", "lat<-0"),
        (
            "boot loop exit inverted",
            "if xn C else FIX",
            "if xn FIX else C",
        ),
        (
            "mode 4 writes R1, not R3",
            "alu=$alu, a=r3, flags<-$fl\n     r3<-bus",
            "alu=$alu, a=r3, flags<-$fl\n     r1<-bus",
        ),
        (
            "RET pops the low byte first",
            "bus<-ram, v1<-bus, lat<-sp+1, sp<-sp+1\n     bus<-ram, v2<-bus, sp<-sp+1",
            "bus<-ram, v2<-bus, lat<-sp+1, sp<-sp+1\n     bus<-ram, v1<-bus, sp<-sp+1",
        ),
        (
            "CMP stores C Z N",
            "alu=sub, a=r1, b=r2, flags<-sef, end",
            "alu=sub, a=r1, b=r2, flags<-czn, end",
        ),
        (
            "ALU flags in the write step (C9)",
            "alu=$alu, a=r1, b=r2, flags<-$fl\n     r1<-bus, bus<-res, end",
            "alu=$alu, a=r1, b=r2\n     r1<-bus, bus<-res, flags<-$fl, end",
        ),
        (
            "two sources on bus_data",
            "a=r1, bus<-a, ram<-bus, lat<-pc, end",
            "a=r1, bus<-a, bus<-res, ram<-bus, lat<-pc, end",
        ),
        (
            "address from bus A in a write step (P4)",
            "bus<-ram, r3<-bus, lat<-pc, end",
            "bus<-ram, r3<-bus, lat<-a:b, end",
        ),
        ("a step without a next step", "$f<-0, end", "$f<-0"),
        (
            "a listed mode missing",
            "instr wsp : imm16",
            "instr wsp : imm8",
        ),
        (
            "fetch shorter than the length (C18)",
            "instr lod : regram\n     f1\n     fv1\n     fv2\n",
            "instr lod : regram\n     f1\n     fv1\n",
        ),
    ];
    for (what, find, replace) in cases {
        println!("{}", caught(what, find, replace));
    }
}
