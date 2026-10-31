//! Programs of several files that must not build: the error messages name
//! the C files and the C names. Needs the compiler tools (compiler/build.cmd).

use hbc2_asm::Target;
use hbc2_cc::{Library, build_library, build_program, report};

/// Writes `files` (name, text) into a folder of its own, and builds them.
fn build(test: &str, files: &[(&str, &str)], library: &Library) -> Result<(), String> {
    let dir = std::env::temp_dir().join("hbc2-cc-build").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let mut sources = Vec::new();
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        sources.push(path);
    }
    build_program(&sources, library, &dir.join("out"), Target::Ram)
        .map(|_| ())
        .map_err(|e| report(&e))
}

/// Built once for all the tests (they run at the same time).
fn library() -> &'static Library {
    static LIBRARY: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    LIBRARY.get_or_init(|| {
        build_library(&std::env::temp_dir().join("hbc2-cc-build-library"))
            .unwrap_or_else(|e| panic!("library: {}", report(&e)))
    })
}

#[test]
fn errors() {
    let library = library();
    let main = ("main.c", "int f(void);\nint main(void) { return f(); }\n");

    // the same global in two files
    let e = build(
        "twice",
        &[
            main,
            ("a.c", "int f(void) { return 1; }\n"),
            ("b.c", "int f(void) { return 2; }\n"),
        ],
        library,
    )
    .unwrap_err();
    assert!(
        e.contains("b.c:1:1: error: 'f' is already defined ("),
        "{e}"
    );
    assert!(e.contains("a.c, line 1)"), "{e}");

    // a global variable too (two values; "int n;" alone is shared)
    let e = build(
        "variable",
        &[
            main,
            (
                "a.c",
                "int f(void) { return 0; }\nint n = 1;\nint puts(const char *s) { return 0; }\n",
            ),
            ("b.c", "int n = 2;\n"),
        ],
        library,
    )
    .unwrap_err();
    assert!(
        e.contains("b.c:1:1: error: 'n' is already defined ("),
        "{e}"
    );
    assert!(e.contains("a.c, line 2)"), "{e}");
    // a library name is not a duplicate: the program's puts replaces it
    assert!(!e.contains("'puts'"), "{e}");

    // used, defined nowhere
    let e = build("missing", &[main], library).unwrap_err();
    assert!(e.contains("main.c:2:1: error: not defined: 'f'"), "{e}");

    // a static function is not seen from another file
    let e = build(
        "static",
        &[
            main,
            (
                "a.c",
                "static int f(void) { return 1; }\nint g(void) { return f(); }\n",
            ),
        ],
        library,
    )
    .unwrap_err();
    assert!(e.contains("main.c:2:1: error: not defined: 'f'"), "{e}");

    // the same static name in two files, the names of the library's statics
    // (printf.c, stdio.c), and a .has file: builds
    build(
        "ok",
        &[
            main,
            ("a.c", "static int n = 1;\nint g(void) { return n; }\n"),
            (
                "b.c",
                "static int n = 2;\nint g(void);\nint f(void) { return n + g(); }\n",
            ),
            ("c.has", "; nothing\nc_label:\n    ret\n"),
            ("d.c", "int field;\nchar *next(void) { return 0; }\n"),
        ],
        library,
    )
    .unwrap();

    // neither .c nor .has
    let e = build("kind", &[("notes.txt", "")], library).unwrap_err();
    assert!(e.contains("not a .c or .has file"), "{e}");
}

/// Compiler errors point at the C line, in every file of the build (a
/// failing file does not stop the others), headers included.
#[test]
fn error_lines() {
    use hbc2_cc::{Diagnostic, Severity};
    let library = library();
    let dir = std::env::temp_dir().join("hbc2-cc-build").join("lines");
    std::fs::create_dir_all(&dir).unwrap();
    let files = [
        (
            "main.c",
            "#include \"defs.h\"\nint main(void)\n{\n  int x = 1\n  return x;\n}\n",
        ),
        ("defs.h", "int f(void);\nint g(void) { return ; }\n"),
        ("a.c", "int f(void)\n{\n  float q;\n  return 0;\n}\n"),
        ("b.c", "int h(void) { return 0; }\n#include \"nothere.h\"\n"),
        ("c.c", "int k(void) { return 0; }\n\n#error stop here\n"),
    ];
    let mut sources = Vec::new();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
        if name.ends_with(".c") {
            sources.push(dir.join(name));
        }
    }
    let errors = match build_program(&sources, library, &dir.join("out"), Target::Ram) {
        Ok(_) => panic!("built"),
        Err(e) => e,
    };
    let at = |name: &str, line: u32| -> Vec<&Diagnostic> {
        errors
            .iter()
            .filter(|d| d.file.as_deref() == Some(&*dir.join(name)) && d.line == line)
            .filter(|d| d.severity == Severity::Error)
            .collect()
    };
    let all = report(&errors);
    // the missing ';' is seen at the next token
    assert!(
        at("main.c", 5)
            .iter()
            .any(|d| d.message.contains("semicolon")),
        "{all}"
    );
    assert!(!at("defs.h", 2).is_empty(), "{all}");
    assert!(
        at("a.c", 3).iter().any(|d| d.message.contains("float")),
        "{all}"
    );
    assert!(
        at("b.c", 2)
            .iter()
            .any(|d| d.message.contains("'nothere.h'")),
        "{all}"
    );
    assert!(
        at("c.c", 3).iter().any(|d| d.message.contains("stop here")),
        "{all}"
    );
}

/// The program's instructions point at their C lines: the function's
/// label at its declaration, each statement, the return at the `}`.
/// `return g;` has no code of its own (copt drops the load of what was
/// just stored), and f has no prologue: lines 5 and 2 hold no instruction.
#[test]
fn c_lines() {
    let dir = std::env::temp_dir().join("hbc2-cc-build").join("c_lines");
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.c");
    std::fs::write(
        &main,
        "int g;\n\
         int f(int x)\n\
         {\n\
         \x20   g = x + 1;\n\
         \x20   return g;\n\
         }\n\
         \n\
         int main(void)\n\
         {\n\
         \x20   return f(41) - 42;\n\
         }\n",
    )
    .unwrap();
    let build = build_program(
        std::slice::from_ref(&main),
        library(),
        &dir.join("out"),
        Target::Ram,
    )
    .unwrap_or_else(|e| panic!("{}", report(&e)));
    let program = &build.program;
    let file = program
        .files
        .iter()
        .position(|f| *f == main)
        .expect("main.c is a file of the program");
    let lines: std::collections::BTreeSet<u32> = program
        .lines
        .iter()
        .filter(|l| l.file == file)
        .map(|l| l.line)
        .collect();
    assert_eq!(lines.into_iter().collect::<Vec<_>>(), [4, 6, 10, 11]);
    let f = program.symbol("_f").unwrap();
    assert_eq!((f.file, f.line), (file, 2));
    let main_label = program.symbol("_main").unwrap();
    assert_eq!((main_label.file, main_label.line), (file, 8));
    // int g; is written at the end of the file, without a line
    let g = program.symbol("_g").unwrap();
    assert_ne!(g.file, file);
    // a breakpoint on a line stops at its first instruction
    let address = program.address_of(file, 10).unwrap();
    assert!(address > f.address);
    // the startup code keeps its own file
    let start = program.symbol("_start").unwrap();
    assert!(program.files[start.file].ends_with("crt0.has"));
}

/// Builds one `main.c` and returns the program.
fn program(test: &str, main: &str) -> hbc2_asm::Program {
    let dir = std::env::temp_dir().join("hbc2-cc-build").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.c");
    std::fs::write(&path, main).unwrap();
    build_program(&[path], library(), &dir.join("out"), Target::Ram)
        .unwrap_or_else(|e| panic!("{}", report(&e)))
        .program
}

/// Unreached code and data are left out (linker, step 11).
#[test]
fn sizes() {
    let has = |p: &hbc2_asm::Program, name: &str| p.symbol(name).is_some();
    let empty = program("empty", "int main(void) { return 0; }\n");
    let hello = program(
        "hello",
        "#include <stdio.h>\nint main(void) { puts(\"hello\"); return 0; }\n",
    );
    let formatted = program(
        "printf",
        "#include <stdio.h>\nint main(void) { printf(\"%d\n\", 42); return 0; }\n",
    );
    let unused = program(
        "unused",
        "static int table[50];\nstatic int count = 3;\n\
         int helper(void) { return table[count]; }\nint main(void) { return 0; }\n",
    );
    for (name, p) in [
        ("empty", &empty),
        ("hello", &hello),
        ("printf", &formatted),
        ("unused", &unused),
    ] {
        println!(
            "{name}: {} bytes, {} removed",
            p.bytes_used,
            p.bytes_removed()
        );
        assert!(has(p, "_start") && has(p, "_main") && has(p, "__exit"));
        let kept: Vec<_> = p
            .symbols
            .iter()
            .filter(|s| s.name.starts_with("__cc"))
            .map(|s| &s.name)
            .collect();
        assert!(kept.is_empty(), "{name}: compare helpers kept: {kept:?}");
    }
    // crt0, device detection, exit and main only
    assert!(empty.bytes_used < 400, "{}", empty.bytes_used);
    assert!(!has(&empty, "_puts") && !has(&empty, "_printf"));
    assert!(has(&hello, "_puts") && !has(&hello, "_printf"));
    assert!(has(&formatted, "_printf") && !has(&formatted, "_puts"));
    assert!(!has(&unused, "_helper"));
    assert_eq!(
        empty.bytes_used, unused.bytes_used,
        "helper and its statics gone"
    );
    assert!(empty.bytes_used < hello.bytes_used && hello.bytes_used < formatted.bytes_used);
}

/// A name defined nowhere fails the build even in a function nothing calls.
#[test]
fn undefined_in_unused_function() {
    let e = build(
        "dead_missing",
        &[(
            "main.c",
            "int g(void);\nint unused(void) { return g(); }\nint main(void) { return 0; }\n",
        )],
        library(),
    )
    .unwrap_err();
    assert!(e.contains("not defined: 'g'"), "{e}");
}

/// Compiles one C file and returns the assembly.
fn compiled(test: &str, source: &str) -> String {
    let dir = std::env::temp_dir().join("hbc2-cc-build").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.c");
    std::fs::write(&path, source).unwrap();
    let out = dir.join("main.has");
    hbc2_cc::compile(&path, &out, 1).unwrap_or_else(|e| panic!("{}", report(&e)));
    std::fs::read_to_string(out).unwrap()
}

/// A volatile variable is loaded again each time (the reload removal
/// leaves it alone); a plain one is not.
#[test]
fn volatile_reloads() {
    let count = |asm: &str, name: &str| {
        asm.lines()
            .filter(|l| l.trim_start().starts_with("lod ") && l.contains(name))
            .count()
    };
    let code = "int f(void) { int a; int b; v = 1; a = v; b = v; return a + b; }\n";
    let plain = compiled("plain", &format!("int v;\n{code}"));
    let vol = compiled("volatile", &format!("volatile int v;\n{code}"));
    assert_eq!(count(&plain, "_v"), 0, "{plain}");
    assert_eq!(count(&vol, "_v"), 4, "{vol}");
    assert!(vol.contains(";volatile _v"), "{vol}");
    // static volatile (renamed after the pass)
    let st = compiled(
        "static_volatile",
        &format!("static volatile int v;\n{code}"),
    );
    assert_eq!(count(&st, "_v"), 4, "{st}");
}

#[test]
fn eeprom_image() {
    let dir = std::env::temp_dir().join("hbc2-cc-build").join("eeprom");
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.c");
    std::fs::write(&main, "int main(void) { return 0; }\n").unwrap();
    // fixed data past the end of the program: not copied by the boot loader
    let fixed = dir.join("fixed.has");
    std::fs::write(&fixed, ".data table $0x8000 1, 2, 3\n.keep table\n").unwrap();
    let build = build_program(&[main, fixed], library(), &dir.join("out"), Target::Eeprom)
        .unwrap_or_else(|e| panic!("{}", report(&e)));
    let program = &build.program;
    let image = build.eeprom_image.as_ref().expect("an EEPROM image");
    assert_eq!(image.len(), 0x10_0000);
    assert_eq!(
        image[..0x1_0000],
        program.image[..],
        "the RAM image at the start"
    );
    assert!(image[0x1_0000..].iter().all(|&b| b == 0));
    let start = program.symbol("_start").unwrap();
    assert!(program.files[start.file].ends_with("boot.has"));
    // the boot loader and the device detection are copied at power-on
    for name in ["_start", "devices_handler", "devices_ignore"] {
        let s = program.symbol(name).unwrap();
        assert!(s.address + s.size <= 0x01FA, "{name} at {:#x}", s.address);
    }
    assert_eq!(program.symbol("_eeprom_port").unwrap().address, 0x01FF);
    let warnings = report(&build.warnings);
    assert!(
        warnings.contains("'table' ($0x8000) is not copied"),
        "{warnings}"
    );
    assert!(
        build_program(
            &[dir.join("main.c")],
            library(),
            &dir.join("out"),
            Target::Ram
        )
        .unwrap()
        .eeprom_image
        .is_none()
    );
}
