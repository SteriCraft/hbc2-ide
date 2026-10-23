//! Drives the whole IDE through egui frames, without a window: open a
//! project, assemble, run, stop at a breakpoint, and see the monitor output.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use hbc2_ide::Ide;
use hbc2_ide::app::{Action, Saved, default_layout};
use hbc2_ide::settings::Settings;
use hbc2_ide::tabs::Tab;
use hbc2_runner::{RunState, Speed};

const PROGRAM: &str = r#".include "interruptHandler.has"

_start:
    mov d, 1        ; monitor DATA_0 (the monitor is plugged first: ports 1-5)
    mov b, 0x0F     ; white on black
    out d, b
    mov d, 2        ; DATA_1
    mov b, 'H'
    out d, b
    mov d, 5        ; CMD
    mov b, 1        ; WRITE
    out d, b
loop:
    jmp loop
"#;

struct Harness {
    ctx: egui::Context,
    ide: Ide,
}

impl Harness {
    fn new() -> Harness {
        let ctx = egui::Context::default();
        let saved = Saved {
            settings: Settings::default(),
            dock: default_layout(),
        };
        let ide = Ide::new(&ctx, Some(saved));
        Harness { ctx, ide }
    }

    fn frame(&mut self) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let ide = &mut self.ide;
        let mut output = self.ctx.run_ui(input, |ui| ide.ui(ui));
        output.textures_delta.clear(); // no GPU here
    }

    fn act(&mut self, action: Action) {
        self.ide.state.actions.push(action);
        self.frame();
    }

    /// Runs frames until `done` holds, or fails after a few seconds.
    fn wait(&mut self, what: &str, done: impl Fn(&Ide) -> bool) {
        let start = Instant::now();
        while !done(&self.ide) {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "timed out waiting for: {what}"
            );
            std::thread::sleep(Duration::from_millis(5));
            self.frame();
        }
    }
}

fn temp_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hbc2-ide-smoke-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    hbc2_ide::project::Project::create(&dir).unwrap();
    std::fs::write(dir.join("main.has"), PROGRAM).unwrap();
    hbc2_ide::project::absolute(&dir).unwrap()
}

fn line_of(text: &str, needle: &str) -> u32 {
    text.lines().position(|l| l.contains(needle)).unwrap() as u32 + 1
}

fn editor_open(ide: &Ide, path: &Path) -> bool {
    ide.dock
        .find_tab(&Tab::Editor(path.to_path_buf()))
        .is_some()
}

#[test]
fn assemble_run_and_break() {
    let dir = temp_project("run");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.frame();

    h.act(Action::OpenProject(dir.clone()));
    assert!(
        editor_open(&h.ide, &main),
        "main.has opens with the project"
    );
    assert!(
        h.ide.dock.find_tab(&Tab::Welcome).is_none(),
        "the start page makes room"
    );

    // Breakpoint on the endless loop, then Run: it assembles by itself first.
    let line = line_of(PROGRAM, "jmp loop");
    h.ide
        .state
        .project_settings
        .breakpoints
        .entry(main.clone())
        .or_default()
        .insert(line);
    h.act(Action::SetSpeed(Speed::Fastest));
    h.act(Action::Run);
    assert!(
        h.ide.state.build.as_ref().is_some_and(|b| b.ok),
        "{:?}",
        h.ide.state.console.messages
    );

    h.wait("the breakpoint", |ide| {
        let s = &ide.state.snapshot;
        s.state == RunState::Paused && s.ticks > 0
    });
    let s = h.ide.state.snapshot.clone();
    let monitor = s.monitor.as_ref().unwrap();
    assert_eq!(monitor.text_buffer()[0].ascii, b'H');
    assert_eq!(monitor.text_buffer()[0].colors, 0x0F);
    assert_eq!(
        h.ide.state.pc_line(&main),
        Some(line),
        "editor shows the current line"
    );
    assert!(
        h.ide
            .state
            .console
            .messages
            .iter()
            .any(|m| m.text.starts_with("Breakpoint at")),
        "the console says why it paused"
    );

    // Step once: the jump goes back to itself, the CPU view sees the change.
    h.act(Action::Step);
    h.wait("the step", |ide| ide.state.snapshot.ticks > s.ticks);
    assert_eq!(h.ide.state.snapshot.state, RunState::Paused);

    // Editing the source marks the build stale; Run keeps the started program.
    let doc = h
        .ide
        .state
        .docs
        .iter_mut()
        .find(|d| d.path == main)
        .unwrap();
    doc.text.push_str("; edited\n");
    h.ide.state.edits += 1;
    assert!(h.ide.state.is_stale());
    assert!(doc_dirty(&h.ide, &main));

    // Assembling uses the unsaved text; saving clears the dirty mark.
    h.act(Action::Assemble);
    assert!(!h.ide.state.is_stale());
    h.wait("the reload at power-on", |ide| {
        ide.state.snapshot.ticks == 0
    });
    h.ide.state.save(std::slice::from_ref(&main));
    assert!(!doc_dirty(&h.ide, &main));
    assert!(
        std::fs::read_to_string(&main)
            .unwrap()
            .ends_with("; edited\n")
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn failed_assembly_does_not_run() {
    let dir = temp_project("fail");
    std::fs::write(dir.join("main.has"), "_start:\n    mov q, 1\n").unwrap();
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::Run);
    h.frame();
    let build = h.ide.state.build.as_ref().unwrap();
    assert!(!build.ok);
    let errors = h.ide.state.diagnostics_for(&dir.join("main.has"));
    assert_eq!(
        errors.first().map(|e| e.0),
        Some(2),
        "error on line 2: {errors:?}"
    );
    assert_eq!(h.ide.state.snapshot.state, RunState::Paused);
    assert!(h.ide.state.emulator.image().is_none(), "nothing was loaded");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unsaved_changes_are_asked_about() {
    let dir = temp_project("unsaved");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.ide.state.docs[0].text.push('x');
    h.act(Action::CloseTab(main.clone()));
    assert!(
        editor_open(&h.ide, &main),
        "not closed: there is a question first"
    );
    assert!(matches!(
        h.ide.state.dialog,
        Some(hbc2_ide::app::Dialog::Unsaved { .. })
    ));

    // "Don't save" then closes it.
    h.ide.state.dialog = None;
    h.ide.state.discard(std::slice::from_ref(&main));
    h.act(Action::CloseTab(main.clone()));
    assert!(!editor_open(&h.ide, &main));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn layout_and_settings_survive_a_restart() {
    let dir = temp_project("restart");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::SetSpeed(Speed::Hz(5_000_000)));
    // eframe stores the state as RON.
    let text = ron::to_string(&h.ide.saved()).unwrap();

    let ctx = egui::Context::default();
    let ide = Ide::new(&ctx, Some(ron::from_str(&text).unwrap()));
    assert_eq!(
        ide.state.project.as_ref().map(|p| p.root.clone()),
        Some(dir.clone())
    );
    assert!(editor_open(&ide, &dir.join("main.has")));
    assert_eq!(Speed::from(ide.state.settings.speed), Speed::Hz(5_000_000));
    std::fs::remove_dir_all(&dir).unwrap();
}

fn doc_dirty(ide: &Ide, path: &Path) -> bool {
    ide.state
        .docs
        .iter()
        .any(|d| d.path == path && d.is_dirty())
}
