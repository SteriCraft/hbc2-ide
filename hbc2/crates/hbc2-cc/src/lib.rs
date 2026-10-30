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
//!
//! Errors and warnings are [`Diagnostic`]s, like the assembler's, about the
//! C file and its line. The program's instructions point at their C lines
//! too (see `lines.rs`), so the debugger works on the C source.

mod lines;
mod messages;

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

pub use hbc2_asm::{Diagnostic, Severity};
use hbc2_asm::{Program, Target, assemble_file};

use lines::LineMaps;
use messages::{c_diagnostic, plain, tool_messages};

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

/// Folder for the compiled files, next to the sources (hidden: the IDE's
/// project view skips folders whose name starts with a dot).
pub const BUILD_DIR: &str = ".build";

/// Unit number of the first library C file: the program's files are 1, 2, …
pub const LIBRARY_FIRST_UNIT: u32 = 100;

/// A path for `.include` in hbc2-asm, where `\` starts an escape.
pub fn include_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Diagnostics as text, one per line (for the command line and tests).
pub fn report(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn io_error(path: &Path, e: std::io::Error) -> Vec<Diagnostic> {
    vec![plain(format!("{}: {e}", path.display()))]
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
/// `compiler/lib/lib.has`. A compiled file newer than its C file, the
/// headers, the compiler and the program running is kept.
pub fn build_library(dir: &Path) -> Result<Library, Vec<Diagnostic>> {
    fs::create_dir_all(dir).map_err(|e| io_error(dir, e))?;
    let src = lib_dir().join("src");
    let mut sources = files_in(&src, "c").map_err(|e| io_error(&src, e))?;
    sources.sort();
    let mut text = format!(
        ".include \"{}\"\n",
        include_path(&lib_dir().join("lib.has"))
    );
    let tools = newest_tool();
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (n, source) in sources.into_iter().enumerate() {
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        let output = dir.join(format!("lib_{stem}.has"));
        let inputs = tools.max(modified(&source));
        if modified(&output) <= inputs
            && let Err(e) = compile(&source, &output, LIBRARY_FIRST_UNIT + n as u32)
        {
            errors.extend(e);
        }
        text += &format!(".include \"{}\"\n", include_path(&output));
        files.push((output, source));
    }
    if errors.iter().any(|d| d.severity == Severity::Error) {
        return Err(errors);
    }
    let path = dir.join("library.has");
    fs::write(&path, text).map_err(|e| io_error(&path, e))?;
    Ok(Library { path, files })
}

/// The files of `dir` with the extension `ext`.
fn files_in(dir: &Path, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect())
}

/// When `path` last changed (the start of time when it does not exist).
fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// The last change to anything a library file's compilation depends on
/// besides itself: the headers, the compiler tools, the peephole rules, and
/// this driver (the program running).
fn newest_tool() -> SystemTime {
    let mut paths = vec![compiler_dir().join("fcc").join("rules.hbc2")];
    for (dir, ext) in [
        (lib_dir().join("include"), "h"),
        (compiler_dir().join("bin"), "exe"),
    ] {
        paths.extend(files_in(&dir, ext).unwrap_or_default());
    }
    paths.extend(std::env::current_exe().ok());
    paths
        .iter()
        .map(|p| modified(p))
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// A program built by [`build_program`].
pub struct Build {
    pub program: Program,
    /// The assembly of each source, in order (a `.has` source is itself).
    pub compiled: Vec<PathBuf>,
    /// The compiler's and the assembler's warnings.
    pub warnings: Vec<Diagnostic>,
}

/// Builds a program from `sources`: C files (`.c`), compiled with the unit
/// numbers 1, 2, … in the order given, and assembly files (`.has`),
/// included as they are. The compiled files and `main.has` (crt0, the
/// runtime, `library`, then the sources) go into `dir`. Every C file is
/// compiled even when one fails, so that all their errors are given; the
/// errors come with the warnings. Messages of the assembler about compiled
/// files name the C files, and C names without their `_`.
pub fn build_program(
    sources: &[PathBuf],
    library: &Library,
    dir: &Path,
) -> Result<Build, Vec<Diagnostic>> {
    fs::create_dir_all(dir).map_err(|e| io_error(dir, e))?;
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
    let mut diagnostics = Vec::new();
    let mut unit = 0;
    for source in sources {
        let output = match source.extension().and_then(|e| e.to_str()) {
            Some("c") => {
                unit += 1;
                let stem = source.file_stem().unwrap_or_default().to_string_lossy();
                let output = dir.join(format!("{stem}_{unit}.has"));
                match compile(source, &output, unit) {
                    Ok(warnings) => diagnostics.extend(warnings),
                    Err(errors) => diagnostics.extend(errors),
                }
                c_files.push((output.clone(), source.clone()));
                output
            }
            Some("has") => source.clone(),
            _ => {
                diagnostics.push(plain(format!(
                    "{}: not a .c or .has file",
                    source.display()
                )));
                continue;
            }
        };
        main += &format!(".include \"{}\"\n", include_path(&output));
        compiled.push(output);
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(diagnostics);
    }
    let main_path = dir.join("main.has");
    fs::write(&main_path, main).map_err(|e| io_error(&main_path, e))?;
    let lines = LineMaps::read(&c_files);
    let in_c = |d: &Diagnostic| c_diagnostic(d.clone(), &c_files, &lines);
    match assemble_file(&main_path, Target::Ram) {
        Ok(mut program) => {
            lines.apply(&mut program);
            diagnostics.extend(program.warnings.iter().map(in_c));
            Ok(Build {
                program,
                compiled,
                warnings: diagnostics,
            })
        }
        Err(errors) => {
            diagnostics.extend(errors.iter().map(in_c));
            Err(diagnostics)
        }
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

/// How a stage went: what it wrote on stderr, and whether it worked.
struct Stage {
    ok: bool,
    messages: String,
}

/// Runs one stage: stdin from `input`, stdout to `output`.
fn stage(program: &Path, args: &[&str], input: &Path, output: &Path) -> Result<Stage, String> {
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
    let mut messages = String::from_utf8_lossy(&result.stderr).trim().to_string();
    if !result.status.success() && messages.is_empty() {
        let name = program.file_stem().unwrap_or_default().to_string_lossy();
        messages = format!("{name} failed");
    }
    Ok(Stage {
        ok: result.status.success(),
        messages,
    })
}

/// Compiles `source` into the assembly file `output`. `unit` is the number
/// of the file in the build (1, 2, …): it goes into the file's labels and
/// `static` names. Gives the warnings, or the errors (with the warnings).
pub fn compile(
    source: &Path,
    output: &Path,
    unit: u32,
) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let temp = |ext: &str| output.with_extension(ext);
    let (pre, tokens, trees, symbols, raw) = (
        temp("i"),
        temp("tok"),
        temp("tree"),
        temp("sym"),
        temp("cc2"),
    );
    let folder = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    // where a file named in a message is looked for
    let include = [folder.clone(), lib_dir().join("include")];
    let mut diagnostics = Vec::new();
    let mut messages = |stage: &Stage| {
        let severity = if stage.ok {
            Severity::Warning
        } else {
            Severity::Error
        };
        diagnostics.extend(tool_messages(&stage.messages, severity, source, &include));
        stage.ok
    };
    let result = (|| -> Result<bool, String> {
        let cpp = Command::new(tool("cpp")?)
            .args(["-E", "-D__hbc2__"])
            // #include "x.h": next to the C file first
            .arg(format!("-I{}", folder.display()))
            .arg(format!("-I{}", include[1].display()))
            .arg(source)
            .stdout(File::create(&pre).map_err(|e| format!("{}: {e}", pre.display()))?)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        let cpp = Stage {
            ok: cpp.status.success(),
            messages: String::from_utf8_lossy(&cpp.stderr).trim().to_string(),
        };
        if !messages(&cpp) {
            return Ok(false);
        }
        let sym = symbols.to_string_lossy().to_string();
        let unit_arg = unit.to_string();
        let rules = compiler_dir().join("fcc").join("rules.hbc2");
        let rules = rules.to_string_lossy();
        let stages: [(&str, &[&str], &Path, &Path); 4] = [
            ("cc0", &[&sym], &pre, &tokens),
            ("cc1.hbc2", &["0", "0"], &tokens, &trees),
            ("cc2.hbc2", &[&sym, "0", "0", "0", &unit_arg], &trees, &raw),
            ("copt", &[&rules], &raw, output),
        ];
        for (name, args, input, out) in stages {
            if !messages(&stage(&tool(name)?, args, input, out)?) {
                return Ok(false);
            }
        }
        let asm = fs::read_to_string(output).map_err(|e| format!("{}: {e}", output.display()))?;
        fs::write(output, rename_statics(&asm, unit))
            .map_err(|e| format!("{}: {e}", output.display()))?;
        Ok(true)
    })();
    for f in [pre, tokens, trees, symbols, raw] {
        let _ = fs::remove_file(f);
    }
    let ok = match result {
        Ok(ok) => ok,
        Err(message) => {
            diagnostics.push(plain(message));
            false
        }
    };
    if ok {
        Ok(diagnostics)
    } else {
        // a later build must not take a half-written file for a fresh one
        let _ = fs::remove_file(output);
        Err(diagnostics)
    }
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
}
