//! C compiler driver for the HBC-2.
//!
//! The compiler is the Fuzix Compiler Kit with an HBC-2 backend, built by
//! `compiler/build.cmd` into `compiler/bin`. [`compile`] runs its stages on
//! one C file and writes the assembly; the program is then assembled with
//! `compiler/runtime/crt0.has` and `compiler/runtime/runtime.has`
//! (see `compiler/ABI.md`).

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The `compiler` folder of the repository (or `HBC2_COMPILER_DIR`).
pub fn compiler_dir() -> PathBuf {
    match std::env::var_os("HBC2_COMPILER_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../compiler"),
    }
}

/// `compiler/runtime`: crt0.has and runtime.has.
pub fn runtime_dir() -> PathBuf {
    compiler_dir().join("runtime")
}

fn tool(name: &str) -> Result<PathBuf, String> {
    let path = compiler_dir().join("bin").join(format!("{name}.exe"));
    if path.exists() {
        Ok(path)
    } else {
        Err(format!(
            "{} not found: build the compiler with compiler/build.cmd",
            path.display()
        ))
    }
}

/// Runs one stage: stdin from `input`, stdout to `output`, error text on failure.
fn stage(program: &Path, args: &[&str], input: &Path, output: &Path) -> Result<(), String> {
    let stdin = File::open(input).map_err(|e| format!("{}: {e}", input.display()))?;
    // cc1 reads back what it wrote: the output must be readable too
    let stdout = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .map_err(|e| format!("{}: {e}", output.display()))?;
    let result = Command::new(program)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    if result.status.success() {
        Ok(())
    } else {
        let name = program.file_stem().unwrap_or_default().to_string_lossy();
        let message = String::from_utf8_lossy(&result.stderr).trim().to_string();
        Err(if message.is_empty() {
            format!("{name} failed")
        } else {
            message
        })
    }
}

/// Compiles `source` into the assembly file `output`. `unit` is the number
/// of the file in the build (1, 2, …): it goes into the file's labels.
pub fn compile(source: &Path, output: &Path, unit: u32) -> Result<(), String> {
    let temp = |ext: &str| output.with_extension(ext);
    let (pre, tokens, trees, symbols) = (temp("i"), temp("tok"), temp("tree"), temp("sym"));
    let result = (|| {
        let cpp = Command::new(tool("cpp")?)
            .args(["-E", "-D__hbc2__"])
            .arg(source)
            .stdout(File::create(&pre).map_err(|e| format!("{}: {e}", pre.display()))?)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        if !cpp.status.success() {
            return Err(String::from_utf8_lossy(&cpp.stderr).trim().to_string());
        }
        let sym = symbols.to_string_lossy().to_string();
        stage(&tool("cc0")?, &[&sym], &pre, &tokens)?;
        stage(&tool("cc1.hbc2")?, &["0", "0"], &tokens, &trees)?;
        let unit = unit.to_string();
        stage(
            &tool("cc2.hbc2")?,
            &[&sym, "0", "0", "0", &unit],
            &trees,
            output,
        )
    })();
    for f in [pre, tokens, trees, symbols] {
        let _ = fs::remove_file(f);
    }
    result
}
