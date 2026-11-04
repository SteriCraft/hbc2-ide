// Bytes a program keeps (the C build's strip) when it calls only the given routines:
// cargo run --release --bin size -- fexp fsin ...
use hbc2_asm::{FileSystem, Options, Target, assemble_with};
fn main() {
    let dir = ftest::work_dir();
    for n in std::env::args().skip(1) {
        let src = format!("_start:\n cal {n}\n hlt\n.include \"{}\"\n", ftest::float_has());
        let p = std::path::Path::new(&dir).join(format!("sz_{n}.has"));
        std::fs::write(&p, src).unwrap();
        let mut o = Options::new(Target::Ram);
        o.strip = true;
        match assemble_with(&p, &FileSystem, o) {
            Ok(pr) => println!("{n:10} {:6} bytes", pr.bytes_used),
            Err(e) => e.iter().for_each(|d| eprintln!("{d}")),
        }
    }
}
