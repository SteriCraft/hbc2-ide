//! C compiler driver for the HBC-2.
//!
//! The compiler is the Fuzix Compiler Kit with an HBC-2 backend, built by
//! `compiler/build.cmd` into `compiler/bin`. [`compile`] runs its stages on
//! one C file (cpp, cc0, cc1, cc2, then copt with the peephole rules of
//! `compiler/fcc/rules.hbc2`), gives its `static` names their own prefix and
//! writes the assembly. [`build_program`] compiles the files of a program
//! and assembles them with `compiler/runtime/crt0.has`,
//! `compiler/runtime/runtime.has` and the C library (see `compiler/ABI.md`).
//! The library is `compiler/lib`: headers in `include` (always on the
//! include path), assembly in `lib.has`, C files in `src` that
//! [`build_library`] compiles.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hbc2_asm::{Program, Target, assemble_file};

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

/// `compiler/lib`: the C library.
pub fn lib_dir() -> PathBuf {
    compiler_dir().join("lib")
}

/// Unit number of the first library C file: the program's files are 1, 2, …
pub const LIBRARY_FIRST_UNIT: u32 = 100;

/// A path for `.include` in hbc2-asm, where `\` starts an escape.
pub fn include_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// The compiled C library.
pub struct Library {
    /// `library.has`: the program includes it after the runtime.
    pub path: PathBuf,
    /// Each compiled file, with its C file.
    pub files: Vec<(PathBuf, PathBuf)>,
}

/// Compiles the C files of the library (`compiler/lib/src/*.c`) into
/// `dir`, and writes `dir/library.has`, which includes them and
/// `compiler/lib/lib.has`.
pub fn build_library(dir: &Path) -> Result<Library, String> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let src = lib_dir().join("src");
    let mut sources: Vec<PathBuf> = fs::read_dir(&src)
        .map_err(|e| format!("{}: {e}", src.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .collect();
    sources.sort();
    let mut text = format!(
        ".include \"{}\"\n",
        include_path(&lib_dir().join("lib.has"))
    );
    let mut files = Vec::new();
    for (n, source) in sources.into_iter().enumerate() {
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        let output = dir.join(format!("lib_{stem}.has"));
        compile(&source, &output, LIBRARY_FIRST_UNIT + n as u32)?;
        text += &format!(".include \"{}\"\n", include_path(&output));
        files.push((output, source));
    }
    let path = dir.join("library.has");
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Library { path, files })
}

/// A program built by [`build_program`].
pub struct Build {
    pub program: Program,
    /// The assembly of each source, in order (a `.has` source is itself).
    pub compiled: Vec<PathBuf>,
}

/// Builds a program from `sources`: C files (`.c`), compiled with the unit
/// numbers 1, 2, … in the order given, and assembly files (`.has`),
/// included as they are. The compiled files and `main.has` (crt0, the
/// runtime, `library`, then the sources) go into `dir`. Error messages
/// name the C files instead of their compiled files, and C names without
/// their `_`.
pub fn build_program(sources: &[PathBuf], library: &Library, dir: &Path) -> Result<Build, String> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let rt = runtime_dir();
    let mut main = String::new();
    for file in [
        rt.join("crt0.has"),
        rt.join("runtime.has"),
        library.path.clone(),
    ] {
        main += &format!(".include \"{}\"\n", include_path(&file));
    }
    // (compiled, C file), for the messages
    let mut c_files = library.files.clone();
    let mut compiled = Vec::new();
    let mut unit = 0;
    for source in sources {
        let output = match source.extension().and_then(|e| e.to_str()) {
            Some("c") => {
                unit += 1;
                let stem = source.file_stem().unwrap_or_default().to_string_lossy();
                let output = dir.join(format!("{stem}_{unit}.has"));
                compile(source, &output, unit)?;
                c_files.push((output.clone(), source.clone()));
                output
            }
            Some("has") => source.clone(),
            _ => return Err(format!("{}: not a .c or .has file", source.display())),
        };
        main += &format!(".include \"{}\"\n", include_path(&output));
        compiled.push(output);
    }
    let main_path = dir.join("main.has");
    fs::write(&main_path, main).map_err(|e| format!("{}: {e}", main_path.display()))?;
    let program = assemble_file(&main_path, Target::Ram).map_err(|errors| {
        errors
            .iter()
            .map(|e| c_message(&e.to_string(), &c_files))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    Ok(Build { program, compiled })
}

/// An assembler message about compiled C files, told in C terms: the C
/// file instead of the compiled one (without a line: a line of the
/// compiled file means nothing in the C file), C names without their `_`.
fn c_message(message: &str, c_files: &[(PathBuf, PathBuf)]) -> String {
    let mut message = message.to_string();
    for (compiled, source) in c_files {
        let compiled = compiled.display().to_string();
        let source = source.display().to_string();
        // "compiled:line:col: error" → "source: error"
        while let Some(start) = message.find(&format!("{compiled}:")) {
            let after = start + compiled.len();
            let digits = message[after..]
                .find(": ")
                .filter(|&n| {
                    message[after..after + n]
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == ':')
                })
                .unwrap_or(0);
            message.replace_range(start..after + digits, &source);
        }
        // "(compiled, line n)" → "(source)"
        while let Some(start) = message.find(&format!("({compiled}, line ")) {
            let end = start + message[start..].find(')').unwrap_or(0);
            message.replace_range(start..=end, &format!("({source})"));
        }
    }
    // names are between quotes: every second part
    let mut out = String::new();
    for (i, part) in message.split('\'').enumerate() {
        if i > 0 {
            out.push('\'');
        }
        out += if i % 2 == 1 { c_name(part) } else { part };
    }
    out.replace("unknown label or variable", "not defined:")
}

/// The C name of an assembly name (`_name`, `__S<unit>_name`), or the name.
fn c_name(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix("__S")
        && let Some(i) = rest.find('_')
        && i > 0
        && rest[..i].chars().all(|c| c.is_ascii_digit())
    {
        return &rest[i + 1..];
    }
    match name.strip_prefix('_') {
        Some(rest) if !rest.is_empty() && !rest.starts_with('_') => rest,
        _ => name,
    }
}

/// The names a compiled file defines without a `; export` line: its C
/// `static` names (`_name` labels, `.data` and `.bss` variables).
fn static_names(asm: &str) -> HashSet<String> {
    let mut exported = HashSet::new();
    let mut defined = HashSet::new();
    for line in asm.lines() {
        let name = if let Some(name) = line.strip_prefix("; export ") {
            exported.insert(name.trim().to_string());
            continue;
        } else if let Some(rest) = line
            .strip_prefix(".data ")
            .or_else(|| line.strip_prefix(".bss "))
        {
            rest.split_whitespace().next().unwrap_or_default()
        } else if let Some(label) = line.strip_suffix(':') {
            label
        } else {
            continue;
        };
        if name.starts_with('_') && !name.starts_with("__") {
            defined.insert(name.to_string());
        }
    }
    defined.retain(|name| !exported.contains(name));
    defined
}

/// Renames the `static` names of a compiled file: `_name` →
/// `__S<unit>_name`. C reserves names that start with `__`, so no C name
/// can be the same. Text in quotes and comments is not changed.
fn rename_statics(asm: &str, unit: u32) -> String {
    let statics = static_names(asm);
    if statics.is_empty() {
        return asm.to_string();
    }
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = String::with_capacity(asm.len() + 64);
    let mut chars = asm.char_indices().peekable();
    let mut quote = None;
    let mut comment = false;
    while let Some((i, c)) = chars.next() {
        if comment {
            comment = c != '\n';
        } else if let Some(q) = quote {
            if c == '\\' {
                out.push(c);
                if let Some((_, next)) = chars.next() {
                    out.push(next);
                }
                continue;
            }
            if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == ';' {
            comment = true;
        } else if ident(c) {
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = chars.peek() {
                if !ident(d) {
                    break;
                }
                end = j + d.len_utf8();
                chars.next();
            }
            let word = &asm[i..end];
            if statics.contains(word) {
                out += &format!("__S{unit}");
            }
            out += word;
            continue;
        }
        out.push(c);
    }
    out
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
/// of the file in the build (1, 2, …): it goes into the file's labels and
/// `static` names.
pub fn compile(source: &Path, output: &Path, unit: u32) -> Result<(), String> {
    let temp = |ext: &str| output.with_extension(ext);
    let (pre, tokens, trees, symbols, raw) = (
        temp("i"),
        temp("tok"),
        temp("tree"),
        temp("sym"),
        temp("cc2"),
    );
    let result = (|| {
        let cpp = Command::new(tool("cpp")?)
            .args(["-E", "-D__hbc2__"])
            // #include "x.h": next to the C file first
            .arg(format!(
                "-I{}",
                source.parent().unwrap_or(Path::new(".")).display()
            ))
            .arg(format!("-I{}", lib_dir().join("include").display()))
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
        let unit_arg = unit.to_string();
        stage(
            &tool("cc2.hbc2")?,
            &[&sym, "0", "0", "0", &unit_arg],
            &trees,
            &raw,
        )?;
        let rules = compiler_dir().join("fcc").join("rules.hbc2");
        stage(&tool("copt")?, &[&rules.to_string_lossy()], &raw, output)?;
        let asm = fs::read_to_string(output).map_err(|e| format!("{}: {e}", output.display()))?;
        fs::write(output, rename_statics(&asm, unit))
            .map_err(|e| format!("{}: {e}", output.display()))
    })();
    for f in [pre, tokens, trees, symbols, raw] {
        let _ = fs::remove_file(f);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASM: &str = r#"; C file 2
.data _count 0x00, 0x03
_helper:
	add b, _count + 1
	lod a, _count.msb ; _count
; export _pub
_pub:
	cal _helper
	lod a, _shared
	cal __T2_3
; export _msg
.data _msg "_count \"_buf", '_', 0
.bss _buf 4
.bss __T2_6 2
"#;

    #[test]
    fn statics() {
        let names = static_names(ASM);
        let mut names: Vec<_> = names.iter().map(String::as_str).collect();
        names.sort();
        assert_eq!(names, ["_buf", "_count", "_helper"]);
    }

    #[test]
    fn renaming() {
        let out = rename_statics(ASM, 2);
        assert!(out.contains(".data __S2_count 0x00, 0x03\n__S2_helper:\n"));
        assert!(out.contains("add b, __S2_count + 1\n\tlod a, __S2_count.msb ; _count\n"));
        assert!(out.contains("cal __S2_helper\n\tlod a, _shared\n\tcal __T2_3\n"));
        assert!(out.contains(".data _msg \"_count \\\"_buf\", '_', 0\n.bss __S2_buf 4\n"));
        assert!(out.contains("; export _pub\n_pub:"));
        assert!(out.contains(".bss __T2_6 2"));
    }

    #[test]
    fn names_in_messages() {
        assert_eq!(c_name("_count"), "count");
        assert_eq!(c_name("__S12_count"), "count");
        assert_eq!(c_name("__mul"), "__mul");
        assert_eq!(c_name("__Sx_a"), "__Sx_a");
        assert_eq!(c_name("label"), "label");
        let files = [(PathBuf::from("/t/a_1.has"), PathBuf::from("/s/a.c"))];
        assert_eq!(
            c_message(
                "/t/a_1.has:12:3: error: unknown label or variable '_foo'",
                &files
            ),
            "/s/a.c: error: not defined: 'foo'"
        );
        assert_eq!(
            c_message(
                "/t/b_2.has:4:1: error: '_g' is already defined (/t/a_1.has, line 7)",
                &files
            ),
            "/t/b_2.has:4:1: error: 'g' is already defined (/s/a.c)"
        );
    }
}
