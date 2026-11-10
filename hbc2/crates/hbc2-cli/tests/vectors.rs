//! `hbc2 vectors`: every generator runs to the end. The generators check the
//! emulator against themselves as they go (assertions), so a change in
//! hbc2-core that breaks one shows here as a panic, not only when the
//! Logisim tests are next run.

use std::path::PathBuf;
use std::process::Command;

fn hbc2(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hbc2")).args(args).output().unwrap()
}

#[test]
fn every_vector_generator_runs() {
    let list = hbc2(&["vectors", "--list"]);
    assert!(list.status.success(), "hbc2 vectors --list failed");
    let modules: Vec<String> = String::from_utf8(list.stdout)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert!(modules.len() >= 18, "only {} modules listed", modules.len());
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    // every generator at once, each in its own process
    let runs: Vec<_> = modules
        .iter()
        .map(|m| {
            let path = dir.join(format!("vectors-{m}.txt"));
            let child = Command::new(env!("CARGO_BIN_EXE_hbc2"))
                .args(["vectors", m, "-o", path.to_str().unwrap()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            (m, path, child)
        })
        .collect();
    let mut failed = Vec::new();
    for (m, path, child) in runs {
        let out = child.wait_with_output().unwrap();
        let rows = std::fs::read_to_string(&path).map(|s| s.lines().count()).unwrap_or(0);
        if !out.status.success() || rows < 2 {
            failed.push(format!("{m}: {} rows, {}", rows, String::from_utf8_lossy(&out.stderr).trim()));
        }
        let _ = std::fs::remove_file(&path);
    }
    assert!(failed.is_empty(), "vector generators failed:\n{}", failed.join("\n"));
}
