//! Programs of several files that must not build: the error messages name
//! the C files and the C names. Needs the compiler tools (compiler/build.cmd).

use hbc2_cc::{Library, build_library, build_program};

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
    build_program(&sources, library, &dir.join("out")).map(|_| ())
}

fn library() -> Library {
    build_library(&std::env::temp_dir().join("hbc2-cc-build-library"))
        .unwrap_or_else(|e| panic!("library: {e}"))
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
        &library,
    )
    .unwrap_err();
    assert!(e.contains("b.c: error: 'f' is already defined ("), "{e}");
    assert!(e.contains("a.c)"), "{e}");

    // a global variable too, and a library name
    let e = build(
        "variable",
        &[
            main,
            (
                "a.c",
                "int f(void) { return 0; }\nint n = 1;\nint puts(const char *s) { return 0; }\n",
            ),
            ("b.c", "int n;\n"),
        ],
        &library,
    )
    .unwrap_err();
    assert!(e.contains("b.c: error: 'n' is already defined ("), "{e}");
    assert!(e.contains("error: 'puts' is already defined ("), "{e}");

    // used, defined nowhere
    let e = build("missing", &[main], &library).unwrap_err();
    assert!(e.contains("main.c: error: not defined: 'f'"), "{e}");

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
        &library,
    )
    .unwrap_err();
    assert!(e.contains("main.c: error: not defined: 'f'"), "{e}");

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
        &library,
    )
    .unwrap();

    // neither .c nor .has
    let e = build("kind", &[("notes.txt", "")], &library).unwrap_err();
    assert!(e.contains("not a .c or .has file"), "{e}");
}
