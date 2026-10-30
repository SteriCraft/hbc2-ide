//! Drives the whole IDE through egui frames, without a window: open a
//! project, assemble, run, stop at a breakpoint, and see the monitor output.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use hbc2_ide::Ide;
use hbc2_ide::app::{Action, Saved, TabGroup};
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
            splits: Vec::new(),
        };
        // As when Windows' accessibility is active: the window then checks
        // that the focused widget is in the accessibility tree.
        ctx.enable_accesskit();
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
        // What the window does with the shapes before drawing them.
        let _ = self
            .ctx
            .tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        check_accessibility(&output.platform_output);
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

/// The focused widget must be in the accessibility tree: AccessKit stops
/// the program otherwise.
fn check_accessibility(output: &egui::PlatformOutput) {
    if let Some(update) = &output.accesskit_update {
        assert!(
            update.nodes.iter().any(|(id, _)| *id == update.focus),
            "the focused widget is not in the accessibility tree"
        );
    }
}

fn temp_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hbc2-ide-smoke-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    hbc2_ide::project::Project::create(&dir, hbc2_ide::project::Language::Assembly, true).unwrap();
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
fn the_editor_follows_a_slow_run() {
    let dir = temp_project("slowrun");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::SetSpeed(Speed::Hz(100)));
    h.act(Action::Run);
    h.wait("the current line while running", |ide| {
        ide.state.snapshot.state == RunState::Running && ide.state.pc_line(&main).is_some()
    });
    assert!(h.ide.state.follows_execution());

    // Faster than 100 Hz, the current line is only shown when paused.
    h.act(Action::SetSpeed(Speed::Hz(1000)));
    assert!(!h.ide.state.follows_execution());
    assert_eq!(h.ide.state.pc_line(&main), None);
    h.act(Action::Pause);
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
fn tab_menu_closes_groups_of_tabs() {
    let dir = temp_project("tabgroups");
    let main = dir.join("main.has");
    let handler = dir.join("interruptHandler.has");
    let other = dir.join("other.has");
    std::fs::write(&other, "").unwrap();
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFile(handler.clone(), None));
    h.act(Action::OpenFile(other.clone(), None));
    let open = |ide: &Ide| -> Vec<bool> {
        [&main, &handler, &other]
            .iter()
            .map(|p| editor_open(ide, p))
            .collect()
    };
    assert_eq!(open(&h.ide), [true, true, true]);

    // Unsaved files stay open.
    let doc = h
        .ide
        .state
        .docs
        .iter_mut()
        .find(|d| d.path == handler)
        .unwrap();
    doc.text.push('x');
    h.act(Action::CloseGroup(TabGroup::Saved));
    assert_eq!(open(&h.ide), [false, true, false]);

    // Tabs: handler, main, other.
    h.act(Action::OpenFile(main.clone(), None));
    h.act(Action::OpenFile(other.clone(), None));
    h.act(Action::CloseGroup(TabGroup::ToTheRight(main.clone())));
    assert_eq!(open(&h.ide), [true, true, false]);

    // Closing the others asks about the unsaved one first.
    h.act(Action::CloseGroup(TabGroup::Others(Some(main.clone()))));
    assert_eq!(open(&h.ide), [true, true, false]);
    assert!(matches!(
        h.ide.state.dialog,
        Some(hbc2_ide::app::Dialog::Unsaved { .. })
    ));
    h.ide.state.dialog = None;
    h.ide.state.discard(std::slice::from_ref(&handler));
    h.act(Action::CloseGroup(TabGroup::Others(Some(main.clone()))));
    assert_eq!(open(&h.ide), [true, false, false]);

    h.act(Action::CloseGroup(TabGroup::All));
    assert_eq!(open(&h.ide), [false, false, false]);
    assert!(h.ide.dock.find_tab(&Tab::Welcome).is_some());
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

#[test]
fn the_layout_stays_when_editors_close() {
    let dir = temp_project("layout");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.frame();
    let tabs =
        |ide: &Ide| -> Vec<Tab> { ide.dock.iter_all_tabs().map(|(_, t)| t.clone()).collect() };
    let start = tabs(&h.ide);
    assert!(start.contains(&Tab::Welcome));

    // Closing the last editor brings the start page back in its place.
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::CloseTab(main.clone()));
    assert_eq!(tabs(&h.ide), start);

    // So does closing the project.
    h.act(Action::OpenFile(main.clone(), None));
    assert!(editor_open(&h.ide, &main));
    h.act(Action::CloseProject);
    assert_eq!(tabs(&h.ide), start, "start page and console stay");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn split_sizes_survive_a_restart() {
    let mut h = Harness::new();
    h.frame();
    let saved = h.ide.saved();
    let mut splits = saved.splits.clone();
    assert!(!splits.is_empty());
    splits[0] = 0.3;
    let ctx = egui::Context::default();
    let mut ide = Ide::new(
        &ctx,
        Some(Saved {
            splits: splits.clone(),
            ..saved
        }),
    );
    assert_eq!(ide.saved().splits, splits);
}

fn doc_dirty(ide: &Ide, path: &Path) -> bool {
    ide.state
        .docs
        .iter()
        .any(|d| d.path == path && d.is_dirty())
}

impl Harness {
    /// One frame with these input events (typing, keys).
    fn frame_with(&mut self, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let ide = &mut self.ide;
        let mut output = self.ctx.run_ui(input, |ui| ide.ui(ui));
        output.textures_delta.clear();
        let _ = self
            .ctx
            .tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        check_accessibility(&output.platform_output);
    }

    /// Characters selected in the editor of `path`.
    fn selection(&self, path: &Path) -> Option<std::ops::Range<usize>> {
        let id = hbc2_ide::views::editor::editor_id(path);
        let range = egui::text_edit::TextEditState::load(&self.ctx, id)?
            .cursor
            .char_range()?
            .as_sorted_char_range();
        Some(range.start.0..range.end.0)
    }
}

fn main_text(ide: &Ide, path: &Path) -> String {
    ide.state
        .docs
        .iter()
        .find(|d| d.path == path)
        .unwrap()
        .text
        .clone()
}

#[test]
fn undo_and_redo_from_the_edit_menu() {
    let dir = temp_project("undo");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    let original = main_text(&h.ide, &main);
    // (undo, redo) available, as the Edit menu shows them.
    let can = |h: &Harness| {
        let docs = &h.ide.state.docs;
        (
            hbc2_ide::app::can_undo(docs, &h.ctx, &main, false),
            hbc2_ide::app::can_undo(docs, &h.ctx, &main, true),
        )
    };

    // Clicking in the editor (moving the cursor) gives nothing to undo.
    let id = hbc2_ide::views::editor::editor_id(&main);
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.frame();
    h.frame_with(vec![egui::Event::Key {
        key: egui::Key::ArrowDown,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    h.frame();
    assert_eq!(can(&h), (false, false));

    // Type in the editor, then undo and redo with the menu actions.
    h.frame_with(vec![egui::Event::Text("xyz".into())]);
    let typed = main_text(&h.ide, &main);
    assert_ne!(typed, original, "the text was typed");
    assert_eq!(can(&h), (true, false));

    h.act(Action::Undo);
    assert_eq!(main_text(&h.ide, &main), original);
    for _ in 0..3 {
        h.frame(); // the editor, focused again, keeps the redo step
    }
    assert_eq!(can(&h), (false, true));
    h.act(Action::Redo);
    assert_eq!(main_text(&h.ide, &main), typed);
    h.frame();
    assert_eq!(can(&h), (true, false));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn find_bar_and_matches() {
    use hbc2_ide::find::FindMode;
    let dir = temp_project("find");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));

    h.act(Action::OpenFind(FindMode::File));
    assert_eq!(h.ide.state.find.mode, Some(FindMode::File));
    h.frame();

    // A "find in files" result opens its file with the match selected.
    let line = line_of(PROGRAM, "jmp loop");
    h.act(Action::ShowMatch {
        path: main.clone(),
        line,
        columns: 8..12,
    });
    h.frame();
    let text = main_text(&h.ide, &main);
    let start: usize = text
        .split('\n')
        .take(line as usize - 1)
        .map(|l| l.chars().count() + 1)
        .sum();
    assert_eq!(h.selection(&main), Some(start + 8..start + 12));
    let selected: String = text.chars().skip(start + 8).take(4).collect();
    assert_eq!(selected, "loop");

    // Ctrl+F then searches for the selected text.
    h.act(Action::OpenFind(FindMode::Files));
    assert_eq!(h.ide.state.find.query, "loop");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn find_as_you_type_then_next() {
    use hbc2_ide::find::FindMode;
    let dir = temp_project("find-next");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFind(FindMode::File));
    h.frame(); // the query field takes the keyboard

    let text = main_text(&h.ide, &main);
    let found = hbc2_ide::find::find_all(&text, "loop", Default::default());
    assert_eq!(found.len(), 2);
    let current = |h: &Harness| h.ide.state.find.current.as_ref().map(|(_, m)| m.clone());

    // Typing goes to the first match.
    h.frame_with(vec![egui::Event::Text("loop".into())]);
    assert_eq!(h.ide.state.find.query, "loop");
    assert_eq!(current(&h), Some(found[0].clone()));

    // Enter goes to the next one, then wraps around.
    let enter = |shift: bool| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        },
    };
    h.frame_with(vec![enter(false)]);
    assert_eq!(current(&h), Some(found[1].clone()));
    h.frame_with(vec![enter(false)]);
    assert_eq!(current(&h), Some(found[0].clone()));
    h.frame_with(vec![enter(true)]);
    assert_eq!(current(&h), Some(found[1].clone()));

    // Escape closes the bar and selects the match in the editor.
    h.frame_with(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    h.frame();
    assert_eq!(h.ide.state.find.mode, None);
    assert_eq!(h.selection(&main), Some(found[1].clone()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn find_in_files_opens_another_file() {
    use hbc2_ide::find::FindMode;
    let dir = temp_project("find-files");
    let other = dir.join("lib.has");
    std::fs::write(&other, "; helpers\n\nprint_text:\n    ret\n").unwrap();
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFind(FindMode::Files));
    h.frame();
    h.frame_with(vec![egui::Event::Text("print_text".into())]);
    let enter = egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    h.frame_with(vec![enter]);
    let hits = h.ide.state.find.results.as_ref().unwrap().hits.clone();
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    h.act(Action::ShowMatch {
        path: hit.path.clone(),
        line: hit.line,
        columns: hit.columns.clone(),
    });
    for _ in 0..5 {
        h.frame();
    }
    assert!(editor_open(&h.ide, &other));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn escape_in_the_editor_closes_the_find_bar() {
    use hbc2_ide::find::FindMode;
    let dir = temp_project("find-escape");
    let main = dir.join("main.has");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFind(FindMode::File));
    h.frame(); // the find bar's field takes the keyboard
    // Then the user clicks in the editor.
    let id = hbc2_ide::views::editor::editor_id(&main);
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.frame();
    assert!(h.ctx.memory(|m| m.has_focus(id)));
    h.frame_with(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert_eq!(h.ide.state.find.mode, None);
    h.frame();
    assert!(
        h.ctx.memory(|m| m.has_focus(id)),
        "the editor keeps the keyboard"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn selecting_past_the_bottom_scrolls() {
    let dir = temp_project("drag-scroll");
    let long = dir.join("long.has");
    let text: String = (1..=600).map(|i| format!("nop ; line {i}\n")).collect();
    std::fs::write(&long, &text).unwrap();
    let mut h = Harness::new();
    // Without a real window, the monitor's window would be drawn over the editor.
    h.ide.state.settings.monitor_window = false;
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFile(long.clone(), None));
    h.frame();

    // Press in the editor's text, near its top, then hold the mouse below it.
    let id = hbc2_ide::views::editor::editor_id(&long);
    let editor = h.ctx.read_response(id).expect("the editor is drawn").rect;
    let view = h.ctx.read_response(id).unwrap().interact_rect;
    let start = egui::pos2(editor.left() + 120.0, view.top() + 30.0);
    let below = egui::pos2(start.x, view.bottom() + 150.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.frame_with(vec![egui::Event::PointerMoved(start), button(start, true)]);
    h.frame_with(vec![egui::Event::PointerMoved(below)]);
    let first = h.selection(&long).expect("a selection").end;
    for _ in 0..60 {
        h.frame();
    }
    let selected = h.selection(&long).unwrap();
    h.frame_with(vec![button(below, false)]);

    // A screen shows about 40 lines; the selection went well past them.
    let line_length = "nop ; line 100\n".len();
    assert!(
        selected.end > first + 40 * line_length,
        "selection {selected:?}, first {first}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn files_and_folders_move() {
    let dir = temp_project("move");
    let other = dir.join("other.has");
    std::fs::write(&other, "; other\n").unwrap();
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    h.act(Action::OpenFile(other.clone(), None));
    let doc = h
        .ide
        .state
        .docs
        .iter_mut()
        .find(|d| d.path == other)
        .unwrap();
    doc.text.push('x');
    h.ide
        .state
        .project_settings
        .breakpoints
        .insert(other.clone(), [1].into());

    // A file into a folder: the open tab, its unsaved text and breakpoints follow.
    let in_lib = dir.join("lib").join("other.has");
    h.act(Action::MovePath(other.clone(), dir.join("lib")));
    assert!(!other.exists() && in_lib.is_file());
    assert!(editor_open(&h.ide, &in_lib) && !editor_open(&h.ide, &other));
    assert!(doc_dirty(&h.ide, &in_lib));
    assert!(
        h.ide
            .state
            .project_settings
            .breakpoints
            .contains_key(&in_lib)
    );
    assert!(
        h.ide
            .state
            .project
            .as_ref()
            .unwrap()
            .files
            .contains(&in_lib)
    );

    // A folder into another, with the file inside.
    let in_src = dir.join("src").join("lib").join("other.has");
    h.act(Action::MovePath(dir.join("lib"), dir.join("src")));
    assert!(in_src.is_file());
    assert!(editor_open(&h.ide, &in_src) && doc_dirty(&h.ide, &in_src));
    assert!(
        h.ide
            .state
            .project_settings
            .breakpoints
            .contains_key(&in_src)
    );

    // Nothing is overwritten.
    std::fs::write(dir.join("other.has"), "").unwrap();
    h.act(Action::MovePath(in_src.clone(), dir.clone()));
    assert!(
        in_src.is_file(),
        "a file with the same name is already there"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A C project: Build saves the files first and runs in the background,
/// compile errors land on their C line, F10 steps whole C lines (into C
/// functions, one instruction in assembly files), and the program prints
/// on the monitor.
#[test]
fn c_project_builds_and_runs() {
    use hbc2_ide::project::{Language, Project};
    let dir = std::env::temp_dir().join(format!("hbc2-ide-smoke-c-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Project::create(&dir, Language::C, false).unwrap();
    let dir = hbc2_ide::project::absolute(&dir).unwrap();
    let main = dir.join("main.c");
    let mut h = Harness::new();
    h.act(Action::OpenProject(dir.clone()));
    assert!(editor_open(&h.ide, &main), "main.c opens with the project");

    let set_text = |h: &mut Harness, text: &str| {
        let doc = h
            .ide
            .state
            .docs
            .iter_mut()
            .find(|d| d.path == main)
            .unwrap();
        doc.text = text.to_string();
        h.ide.state.edits += 1;
    };
    set_text(&mut h, "int main(void)\n{\n    return x;\n}\n");
    h.act(Action::Assemble);
    assert!(
        h.ide.state.is_building(),
        "the build runs in the background"
    );
    h.wait("the build", |ide| !ide.state.is_building());
    let build = h.ide.state.build.as_ref().unwrap();
    assert!(!build.ok);
    let errors = h.ide.state.diagnostics_for(&main);
    assert!(
        errors.iter().any(|e| e.0 == 3 && e.1),
        "error on line 3: {errors:?}"
    );
    assert!(!doc_dirty(&h.ide, &main), "saved before the build");
    assert!(
        std::fs::read_to_string(&main)
            .unwrap()
            .contains("return x;")
    );

    set_text(
        &mut h,
        "#include <stdio.h>\n\
         int add(int a, int b) { return a + b; }\n\
         int main(void)\n\
         {\n\
         \x20   int x = 1;\n\
         \x20   x = x + 2;\n\
         \x20   x = add(x, 4);\n\
         \x20   puts(\"Hi\");\n\
         \x20   return x;\n\
         }\n",
    );
    // No RTC: its interrupt would take a step into the interrupt handler.
    h.act(Action::SetPeripherals(hbc2_ide::emulator::Peripherals {
        monitor: true,
        rtc: false,
        keyboard: false,
    }));
    // A breakpoint on a C line: Run builds, then stops there, and the
    // editor shows the C line.
    h.ide
        .state
        .project_settings
        .breakpoints
        .entry(main.clone())
        .or_default()
        .insert(6);
    h.act(Action::SetSpeed(Speed::Fastest));
    h.act(Action::Run);
    h.wait("the breakpoint on the C line", |ide| {
        let s = &ide.state.snapshot;
        !ide.state.is_building() && s.state == RunState::Paused && s.ticks > 0
    });
    assert!(
        h.ide.state.build.as_ref().is_some_and(|b| b.ok),
        "{:?}",
        h.ide.state.console.messages
    );
    assert_eq!(h.ide.state.pc_line(&main), Some(6));
    let code_lines = &h.ide.state.build.as_ref().unwrap().code_lines;
    assert!(code_lines.get(&main).is_some_and(|l| l.contains(&6)));

    // F10: the whole line, into a C function, out of it, then into puts,
    // which is assembly: one instruction at a time there.
    let step = |h: &mut Harness| {
        let ticks = h.ide.state.snapshot.ticks;
        h.act(Action::Step);
        h.wait("the step", |ide| {
            let s = &ide.state.snapshot;
            s.state == RunState::Paused && s.ticks > ticks
        });
        h.ide.state.snapshot.ticks - ticks
    };
    step(&mut h);
    assert_eq!(h.ide.state.pc_line(&main), Some(7), "line 6 in one step");
    step(&mut h);
    assert_eq!(h.ide.state.pc_line(&main), Some(2), "into add");
    step(&mut h);
    assert_eq!(h.ide.state.pc_line(&main), Some(7), "back in main");
    step(&mut h);
    assert_eq!(h.ide.state.pc_line(&main), Some(8));
    step(&mut h);
    let pc = h.ide.state.snapshot.cpu.pc;
    let (file, _) = h.ide.state.source_location(pc).unwrap();
    assert!(file.ends_with("stdio.has"), "in puts: {}", file.display());
    let ticks = step(&mut h);
    assert!(ticks <= 4, "one instruction in assembly ({ticks} cycles)");

    h.act(Action::Run);
    h.wait("the text on the monitor", |ide| {
        ide.state.snapshot.monitor.as_ref().is_some_and(|m| {
            let text = m.text_buffer();
            text[0].ascii == b'H' && text[1].ascii == b'i'
        })
    });
    h.act(Action::Pause);

    let compiled = dir.join(".build").join("main_1.has");
    assert!(compiled.is_file());
    assert!(
        h.ide.state.is_generated(&compiled),
        "compiled files are read-only"
    );
    assert!(!h.ide.state.is_generated(&main));
    assert!(
        !h.ide
            .state
            .project
            .as_ref()
            .unwrap()
            .files
            .contains(&compiled),
        "the build folder is not in the project view"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
