//! `hbc2 compare`: output and exit codes.

use std::path::PathBuf;
use std::process::Command;

/// The expected trace of hbc2-asm's loop test.
fn trace() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hbc2-asm/tests/trace/loop.csv"
    );
    std::fs::read_to_string(path).unwrap()
}

fn write(name: &str, content: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Runs `hbc2 compare a b`: exit code and stdout.
fn compare(a: &str, b: &str) -> (i32, String) {
    // One pair of files per call: the tests run in parallel.
    let (a, b) = (
        write(&format!("compare-{a}-vs-{b}-1"), &trace_variant(a)),
        write(&format!("compare-{a}-vs-{b}-2"), &trace_variant(b)),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_hbc2"))
        .arg("compare")
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn trace_variant(name: &str) -> String {
    let trace = trace();
    match name {
        "same.csv" | "copy.csv" => trace,
        // Line n=13: A is 01 after `sub a, 1`; make it 07.
        "register.csv" => trace.replace("13,0010,3440,01,00,10,5A,", "13,0010,3440,07,00,10,5A,"),
        "short.csv" => trace.lines().take(12).map(|l| format!("{l}\n")).collect(),
        _ => unreachable!(),
    }
}

#[test]
fn identical_traces() {
    let (code, out) = compare("same.csv", "copy.csv");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("identical"), "{out}");
}

#[test]
fn one_register_differs() {
    let (code, out) = compare("same.csv", "register.csv");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with("traces differ at n=13 (line 15 of the files)\n"),
        "{out}"
    );
    // Values are padded to the width of the first file's name.
    let row = out.lines().nth(2).unwrap();
    assert!(
        row.starts_with("  a       01 ") && row.ends_with("  |  07"),
        "{out}"
    );
    assert_eq!(
        out.lines().nth(1).unwrap().find('|'),
        row.find('|'),
        "{out}"
    );
    let before = "previous lines:\n  10,0018,5DC0000C,";
    assert!(out.contains(before), "{out}");
    assert!(
        out.trim_end()
            .ends_with("12,000E,4050,02,00,10,5A,00,00,00,00,FDFE,14,,10=02"),
        "{out}"
    );
}

#[test]
fn one_trace_shorter() {
    let (code, out) = compare("short.csv", "same.csv");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with("traces differ at n=11 (line 13 of the files)\n"),
        "{out}"
    );
    assert!(out.contains("ends here; "), "{out}");
    assert!(out.contains("\n  11,000C,8440,"), "{out}");

    let (code, _) = compare("same.csv", "short.csv");
    assert_eq!(code, 1);
}

#[test]
fn missing_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_hbc2"))
        .args(["compare", "no_such_a.csv", "no_such_b.csv"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
