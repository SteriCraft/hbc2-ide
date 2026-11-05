//! The shipped microcode builds, passes every rule and fits.

#[test]
fn builds() {
    let m = match hbc2_ucode::build() {
        Ok(m) => m,
        Err(e) => panic!("{} error(s):\n{}", e.len(), e.join("\n")),
    };
    for (what, n) in &m.checks {
        println!("{n:6}  {what}");
    }
    assert_eq!(m.images().len(), 7);
}
