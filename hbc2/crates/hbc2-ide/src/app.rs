//! The IDE's state and everything it can do.
//!
//! Views and menus never change the state directly when it involves more
//! than their own widget: they queue an [`Action`], and the actions run once
//! the frame's UI is built. This keeps borrowing simple (a view only sees
//! what it shows) and gives one place where each operation is written.

use std::collections::{BTreeSet, HashMap};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use egui_dock::{DockState, Node, NodeIndex};
use hbc2_asm::source::normalize;
use hbc2_asm::{Diagnostic, Program, Severity, SourceProvider, Target};
use hbc2_runner::{RunState, Snapshot, Speed};
use serde::{Deserialize, Serialize};

use crate::console::{Console, Level};
use crate::document::{Document, shift_breakpoints};
use crate::emulator::{Emulator, Image, Peripherals, ram_address};
use crate::find::{Find, FindMode};
use crate::project::{self, Language, Project};
use crate::settings::{ProjectSettings, Settings, TargetSetting};
use crate::tabs::Tab;
use crate::views::disasm::DisasmView;
use crate::views::editor::{char_index_of_line, editor_id};
use crate::views::memory::MemoryView;
use crate::views::monitor::MonitorView;

/// Saved between sessions. The layout is fixed: only the sizes of its parts
/// are kept.
#[derive(Serialize, Deserialize)]
pub struct Saved {
    pub settings: Settings,
    /// Fractions of the layout's splits, in the order of `default_layout`.
    #[serde(default)]
    pub splits: Vec<f32>,
}

#[derive(Clone, Debug)]
pub enum Action {
    NewProject,
    /// Creates a project in this folder and opens it. The flag adds the
    /// default interrupt handler (assembly only).
    CreateProject(PathBuf, Language, bool),
    OpenProjectDialog,
    OpenProject(PathBuf),
    CloseProject,
    OpenFileDialog,
    OpenFile(PathBuf, Option<u32>),
    NewFile(PathBuf),
    NewFolder(PathBuf),
    RenameFile(PathBuf),
    DeleteFile(PathBuf),
    RefreshFiles,
    /// Opens the file manager on a folder, or on a file's folder.
    ShowFolder(PathBuf),
    /// Moves a file or folder into another folder.
    MovePath(PathBuf, PathBuf),
    Save,
    SaveAll,
    /// Closes an editor tab, asking about unsaved changes.
    CloseTab(PathBuf),
    CloseCurrentTab,
    /// Closes several editor tabs, asking about unsaved changes once.
    CloseTabs(Vec<PathBuf>),
    CloseGroup(TabGroup),
    /// Undoes or redoes the last edit in the editor tab.
    Undo,
    Redo,
    /// Opens the find bar.
    OpenFind(FindMode),
    /// Closes the find bar; the editor gets the keyboard, with the current
    /// match selected.
    CloseFind,
    /// Opens a file and selects a "find in files" match.
    ShowMatch {
        path: PathBuf,
        line: u32,
        columns: Range<usize>,
    },
    Assemble,
    Run,
    Pause,
    Step,
    Reset,
    ExportImage,
    LoadImage,
    SaveEeprom,
    SetTarget(TargetSetting),
    /// Leave out unused code in an assembly project, or keep everything.
    SetStrip(bool),
    SetPeripherals(Peripherals),
    SetSpeed(Speed),
    /// Breakpoint lines changed: send the new addresses to the emulator.
    BreakpointsChanged,
    ToggleAddressBreakpoint(u16),
    /// Sends the RTC's interrupt now.
    RtcInterrupt,
    Quit,
    ShowDialog(Dialog),
}

/// Editor tabs closed together, from the tab menu or the keyboard.
#[derive(Clone, Debug)]
pub enum TabGroup {
    /// The editor tabs after this one.
    ToTheRight(PathBuf),
    /// The editor tabs without unsaved changes.
    Saved,
    /// All editor tabs but this one (the current editor if none).
    Others(Option<PathBuf>),
    All,
}

#[derive(Clone, Debug)]
pub enum Dialog {
    /// Unsaved changes in `paths`; `then` runs after saving or discarding them.
    Unsaved {
        paths: Vec<PathBuf>,
        then: Box<Action>,
    },
    NewProject {
        /// The project goes in `parent/name`.
        parent: PathBuf,
        name: String,
        language: Language,
        interrupt_handler: bool,
        error: Option<String>,
    },
    NewFile {
        dir: PathBuf,
        name: String,
        error: Option<String>,
    },
    NewFolder {
        dir: PathBuf,
        name: String,
        error: Option<String>,
    },
    Rename {
        path: PathBuf,
        name: String,
        error: Option<String>,
    },
    Delete {
        path: PathBuf,
    },
    Message {
        title: String,
        text: String,
    },
    About,
    Keyboard,
}

/// A program and its warnings, or the errors (with the warnings).
type BuildResult = Result<(Program, Vec<Diagnostic>), Vec<Diagnostic>>;

/// A C build running in the background.
pub struct PendingBuild {
    result: std::sync::mpsc::Receiver<BuildResult>,
    started: Instant,
    /// `State::edits` when the build started (the files were saved then).
    edits: u64,
    /// What to do once the build is loaded (Run or Step pressed meanwhile).
    then: Option<Action>,
}

/// Result of the last assembly.
pub struct Build {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    /// Lines holding an instruction, per file.
    pub code_lines: HashMap<PathBuf, BTreeSet<u32>>,
}

pub struct State {
    pub settings: Settings,
    pub project: Option<Project>,
    pub project_settings: ProjectSettings,
    pub docs: Vec<Document>,
    pub console: Console,
    pub emulator: Emulator,
    pub build: Option<Build>,
    /// The C build in progress, if any.
    pub building: Option<PendingBuild>,
    /// Counts edits to the sources (to know when the loaded build is out of date).
    pub edits: u64,
    /// Value of `edits` when the loaded image was assembled (`None`: not assembled here).
    pub loaded_edits: Option<u64>,
    /// Latest emulator state, read once per frame.
    pub snapshot: Arc<Snapshot>,
    pub dialog: Option<Dialog>,
    pub actions: Vec<Action>,
    pub monitor_view: MonitorView,
    pub memory_view: MemoryView,
    pub disasm_view: DisasmView,
    pub find: Find,
    /// The file of the editor tab shown in the editor group, if any.
    pub active_editor: Option<PathBuf>,
    /// (cycles, PC) the editor last jumped to.
    followed: Option<(u64, u16)>,
    /// Last breakpoint stop written to the console.
    reported_stop: Option<(u64, hbc2_core::StopReason)>,
    last_disk_check: f64,
    quit_confirmed: bool,
    title: String,
    /// Width the "CPU & I/O" tab needs to show all its text: the fixed
    /// width of the right-hand group.
    pub tools_min_width: f32,
}

pub struct Ide {
    pub dock: DockState<Tab>,
    pub state: State,
    /// Ctrl+W is held: true once R or K made it Ctrl+W+R or Ctrl+W+K.
    close_chord: Option<bool>,
}

/// Open editor buffers first, files on disk otherwise: assembling uses
/// unsaved changes without saving them.
struct Sources<'a>(HashMap<PathBuf, &'a str>);

impl SourceProvider for Sources<'_> {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.0.get(&normalize(path)) {
            Some(text) => Ok(text.to_string()),
            None => std::fs::read_to_string(path),
        }
    }
}

pub fn default_layout() -> DockState<Tab> {
    let mut dock = DockState::new(vec![Tab::Welcome]);
    let surface = dock.main_surface_mut();
    // The fraction is the share of the left or top part.
    let [center, _] = surface.split_left(NodeIndex::root(), 0.16, vec![Tab::Project]);
    let [center, _] = surface.split_right(center, 0.56, TOOL_TABS.to_vec());
    surface.split_below(center, 0.74, vec![Tab::Console]);
    dock
}

/// Tabs of the right-hand group, which share its width equally.
/// Up to this speed, the editor, disassembly and memory views follow the PC while running.
pub const FOLLOW_MAX_HZ: u64 = 100;

pub const TOOL_TABS: [Tab; 3] = [Tab::Cpu, Tab::Disassembly, Tab::Memory];

/// Fractions of the splits (the sizes the user set with the mouse).
fn split_fractions(dock: &DockState<Tab>) -> Vec<f32> {
    dock.main_surface()
        .iter()
        .filter_map(|node| match node {
            Node::Horizontal(split) | Node::Vertical(split) => Some(split.fraction),
            _ => None,
        })
        .collect()
}

/// The fixed layout, with the split fractions of an earlier session when
/// they fit it.
fn layout_with(splits: &[f32]) -> DockState<Tab> {
    let mut dock = default_layout();
    if splits.len() != split_fractions(&dock).len()
        || splits.iter().any(|f| !(0.0..=1.0).contains(f))
    {
        return dock;
    }
    let splits_in_layout = dock
        .main_surface_mut()
        .iter_mut()
        .filter_map(|node| match node {
            Node::Horizontal(split) | Node::Vertical(split) => Some(split),
            _ => None,
        });
    for (split, &fraction) in splits_in_layout.zip(splits) {
        split.fraction = fraction;
    }
    dock
}

impl Ide {
    pub fn new(ctx: &egui::Context, saved: Option<Saved>) -> Ide {
        let (settings, dock) = match saved {
            Some(saved) => (saved.settings, layout_with(&saved.splits)),
            None => (Settings::default(), default_layout()),
        };
        let repaint = ctx.clone();
        let emulator = Emulator::new(Peripherals::default(), settings.speed.into(), move || {
            repaint.request_repaint()
        });
        let snapshot = emulator.runner.snapshot();
        let mut ide = Ide {
            dock,
            close_chord: None,
            state: State {
                settings,
                project: None,
                project_settings: ProjectSettings::default(),
                docs: Vec::new(),
                console: Console::default(),
                emulator,
                build: None,
                building: None,
                edits: 0,
                loaded_edits: None,
                snapshot,
                dialog: None,
                actions: Vec::new(),
                monitor_view: MonitorView::default(),
                memory_view: MemoryView::default(),
                disasm_view: DisasmView::default(),
                find: Find::default(),
                active_editor: None,
                followed: None,
                reported_stop: None,
                last_disk_check: 0.0,
                quit_confirmed: false,
                title: String::new(),
                tools_min_width: 0.0,
            },
        };
        ide.state.disasm_view.follow_pc = ide.state.settings.follow_pc;
        ide.state.memory_view.follow_pc = ide.state.settings.follow_pc;
        ide.state
            .console
            .info("HBC-2 IDE ready. F7 assembles, F5 runs, F10 steps.");

        // Reopen the last project, with the files that were open in it.
        if let Some(root) = ide.state.settings.last_project.clone() {
            ide.open_project(&root, true);
        }
        ide
    }

    /// What to save between sessions.
    pub fn saved(&mut self) -> Saved {
        self.store_project_settings();
        Saved {
            settings: self.state.settings.clone(),
            splits: split_fractions(&self.dock),
        }
    }

    fn store_project_settings(&mut self) {
        let Some(project) = &self.state.project else {
            return;
        };
        self.state.project_settings.open_files = self.editor_paths();
        // Forget breakpoint sets that became empty.
        self.state
            .project_settings
            .breakpoints
            .retain(|_, lines| !lines.is_empty());
        self.state
            .settings
            .projects
            .insert(project.root.clone(), self.state.project_settings.clone());
    }

    fn editor_paths(&self) -> Vec<PathBuf> {
        self.dock
            .iter_all_tabs()
            .filter_map(|(_, t)| match t {
                Tab::Editor(path) => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// One frame of the whole IDE.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.state.snapshot = self.state.emulator.runner.snapshot();
        self.state.poll_build(&ctx);
        self.shortcuts(&ctx);
        self.watch_emulator();
        self.check_disk(&ctx);
        self.handle_close_request(&ctx);
        self.state.active_editor = self.active_editor();

        crate::chrome::menu_bar(ui, &mut self.state);
        crate::chrome::toolbar(ui, &mut self.state);
        crate::chrome::status_bar(ui, &self.state);
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(0.0))
            .show(ui, |ui| {
                self.fix_tools_width();
                // Width of the right-hand group, as laid out last frame.
                let tools_width = self
                    .dock
                    .find_tab(&TOOL_TABS[0])
                    .and_then(|path| self.dock.leaf(path.node_path()).ok())
                    .map_or(0.0, |leaf| leaf.rect.width());
                let mut viewer = crate::tabs::Viewer {
                    state: &mut self.state,
                    tools_width,
                };
                // Tabs cannot be moved, and only editor tabs can be closed;
                // the separators still resize the groups.
                egui_dock::DockArea::new(&mut self.dock)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .draggable_tabs(false)
                    .tab_context_menus(false)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_inside(ui, &mut viewer);
            });
        self.hide_fixed_separator_cursor(&ctx);
        crate::chrome::dialogs(&ctx, &mut self.state);
        self.monitor_window(&ctx);

        while !self.state.actions.is_empty() {
            let actions = std::mem::take(&mut self.state.actions);
            for action in actions {
                self.perform(action, &ctx);
            }
        }
        self.update_title(&ctx);
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut as K, Modifiers as M};
        let table = [
            (K::new(M::NONE, Key::F7), Action::Assemble),
            (K::new(M::COMMAND, Key::B), Action::Assemble),
            (K::new(M::NONE, Key::F5), Action::Run),
            (K::new(M::NONE, Key::F6), Action::Pause),
            (K::new(M::NONE, Key::F10), Action::Step),
            (K::new(M::COMMAND, Key::F5), Action::Reset),
            (K::new(M::COMMAND | M::SHIFT, Key::S), Action::SaveAll),
            (K::new(M::COMMAND, Key::S), Action::Save),
            (K::new(M::COMMAND, Key::O), Action::OpenProjectDialog),
            (
                K::new(M::COMMAND | M::SHIFT, Key::F),
                Action::OpenFind(FindMode::Files),
            ),
            (K::new(M::COMMAND, Key::F), Action::OpenFind(FindMode::File)),
        ];
        for (shortcut, action) in table {
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                self.state.actions.push(action);
            }
        }
        // Ctrl+W closes the tab when W is released; R or K pressed while
        // Ctrl+W is held closes the other tabs or all of them instead.
        if ctx.input_mut(|i| i.consume_shortcut(&K::new(M::COMMAND, Key::W))) {
            self.close_chord.get_or_insert(false);
        }
        if let Some(chord) = &mut self.close_chord {
            let (others, all) = ctx.input_mut(|i| {
                (
                    i.consume_key(M::COMMAND, Key::R),
                    i.consume_key(M::COMMAND, Key::K),
                )
            });
            if others {
                *chord = true;
                self.state
                    .actions
                    .push(Action::CloseGroup(TabGroup::Others(None)));
            }
            if all {
                *chord = true;
                self.state.actions.push(Action::CloseGroup(TabGroup::All));
            }
            if !ctx.input(|i| i.key_down(Key::W)) {
                if !*chord {
                    self.state.actions.push(Action::CloseCurrentTab);
                }
                self.close_chord = None;
            }
        }
        // Escape closes the find bar, from the editor too (the find bar's own
        // field handles it by itself; the monitor sends it to the HBC-2).
        let editor_focused = |m: &egui::Memory| {
            self.state
                .active_editor
                .as_ref()
                .is_some_and(|p| m.has_focus(editor_id(p)))
        };
        if self.state.find.mode.is_some()
            && ctx.memory(|m| m.focused().is_none() || editor_focused(m))
            && ctx.input_mut(|i| i.consume_key(M::NONE, Key::Escape))
        {
            self.state.actions.push(Action::CloseFind);
        }
        // A focused text field undoes its own edits; with nothing focused,
        // undo works on the editor tab.
        if ctx.memory(|m| m.focused().is_none()) {
            let table = [
                (K::new(M::COMMAND | M::SHIFT, Key::Z), Action::Redo),
                (K::new(M::COMMAND, Key::Y), Action::Redo),
                (K::new(M::COMMAND, Key::Z), Action::Undo),
            ];
            for (shortcut, action) in table {
                if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                    self.state.actions.push(action);
                }
            }
        }
    }

    /// Reacts to the emulator pausing by itself, and follows the PC in the editor.
    fn watch_emulator(&mut self) {
        let s = self.state.snapshot.clone();
        // A new stop has a new cycle count (a run can reach a breakpoint before
        // any "running" snapshot is seen).
        if let Some(stop @ hbc2_core::StopReason::Breakpoint(address)) = s.stop
            && self.state.reported_stop != Some((s.ticks, stop))
        {
            self.state.reported_stop = Some((s.ticks, stop));
            let place = self.state.source_location(address).map(|(path, line)| {
                format!(
                    " ({}:{line})",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            });
            self.state.console.info(format!(
                "Breakpoint at {address:04X}{} after {} cycles.",
                place.unwrap_or_default(),
                crate::views::cpu::group_digits(s.ticks)
            ));
        }
        if s.ticks > 0 && self.state.settings.follow_pc && self.state.follows_execution() {
            let key = (s.ticks, s.cpu.pc);
            let running = s.state == RunState::Running;
            // While running, the cycle count changes with every snapshot:
            // only a new PC moves the editor.
            let new = match self.state.followed {
                Some((_, pc)) if running => pc != s.cpu.pc,
                followed => followed != Some(key),
            };
            if new {
                self.state.followed = Some(key);
                if let Some((path, line)) = self.state.source_location(s.cpu.pc) {
                    self.open_file(&path, Some(line));
                    if let Some(doc) = self.state.docs.iter_mut().find(|d| d.path == path) {
                        doc.scroll_gently = running;
                    }
                }
            }
        }
    }

    /// Every second or so: reload files changed on disk, refresh the file list.
    fn check_disk(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if now - self.state.last_disk_check < 1.5 {
            return;
        }
        self.state.last_disk_check = now;
        for doc in &mut self.state.docs {
            let was_conflict = doc.conflict;
            if doc.check_disk() {
                self.state.edits += 1;
                self.state
                    .console
                    .info(format!("Reloaded {} (changed on disk).", doc.name()));
            } else if doc.conflict && !was_conflict {
                self.state.console.warning(format!(
                    "{} changed on disk, and has unsaved changes here.",
                    doc.name()
                ));
            }
        }
        if let Some(project) = &mut self.state.project {
            project.rescan();
        }
    }

    /// Gives the right-hand group exactly the width the "CPU & I/O" tab
    /// needs. Done before the dock is drawn, it also undoes any drag of the
    /// separator, so that group's width is fixed.
    fn fix_tools_width(&mut self) {
        let need = self.state.tools_min_width;
        if need <= 0.0 {
            return; // not measured yet
        }
        let Some((node, _)) = self.dock.find_main_surface_tab(&TOOL_TABS[0]) else {
            return;
        };
        let Some(parent) = node.parent().filter(|_| node.is_right()) else {
            return;
        };
        if let Node::Horizontal(split) = &mut self.dock.main_surface_mut()[parent] {
            let total = split.rect.width();
            if total > 0.0 {
                split.fraction = ((total - need) / total).clamp(0.05, 0.95);
            }
        }
    }

    /// The separator left of the right-hand group cannot be dragged: no
    /// resize cursor over it.
    fn hide_fixed_separator_cursor(&self, ctx: &egui::Context) {
        let Some(rect) = self
            .dock
            .find_tab(&TOOL_TABS[0])
            .and_then(|path| self.dock.leaf(path.node_path()).ok())
            .map(|leaf| leaf.rect)
        else {
            return;
        };
        let Some(pointer) = ctx.pointer_hover_pos() else {
            return;
        };
        let near = (pointer.x - rect.left()).abs() <= 8.0;
        if near && (rect.top()..=rect.bottom()).contains(&pointer.y) {
            ctx.set_cursor_icon(egui::CursorIcon::Default);
        }
    }

    /// The HBC-2 monitor, in a window of its own.
    fn monitor_window(&mut self, ctx: &egui::Context) {
        if !self.state.settings.monitor_window {
            return;
        }
        let state = &mut self.state;
        let snapshot = &state.snapshot;
        let builder = egui::ViewportBuilder::default()
            .with_title("HBC-2 Monitor")
            .with_inner_size([600.0, 480.0])
            .with_min_inner_size([300.0, 250.0])
            .with_icon(crate::icon());
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("hbc2-monitor"),
            builder,
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    state.monitor_view.ui(
                        ui,
                        snapshot.monitor.as_ref(),
                        snapshot.keyboard.is_some(),
                        &state.emulator.runner,
                    );
                });
                if ui.input(|i| i.viewport().close_requested()) {
                    state.settings.monitor_window = false;
                }
            },
        );
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.state.quit_confirmed {
            return;
        }
        let dirty = self.state.dirty_paths();
        if !dirty.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.state.dialog = Some(Dialog::Unsaved {
                paths: dirty,
                then: Box::new(Action::Quit),
            });
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let dirty = if self.state.dirty_paths().is_empty() {
            ""
        } else {
            " •"
        };
        let title = match &self.state.project {
            Some(p) => format!("{}{dirty} — HBC-2 IDE", p.name()),
            None => "HBC-2 IDE".to_string(),
        };
        if title != self.state.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.state.title = title;
        }
    }

    // ---------------------------------------------------------------- actions

    pub fn perform(&mut self, action: Action, ctx: &egui::Context) {
        let state = &mut self.state;
        match action {
            Action::NewProject => {
                if state.guard_unsaved(state.dirty_paths(), Action::NewProject) {
                    return;
                }
                state.dialog = Some(Dialog::NewProject {
                    parent: project::default_projects_parent(),
                    name: "newProject".into(),
                    language: Language::Assembly,
                    interrupt_handler: true,
                    error: None,
                });
            }
            Action::CreateProject(dir, language, interrupt_handler) => {
                match Project::create(&dir, language, interrupt_handler) {
                    Ok(project) => {
                        let root = project.root.clone();
                        self.open_project(&root, true);
                    }
                    Err(e) => state.message("Cannot create the project", e.to_string()),
                }
            }
            Action::OpenProjectDialog => {
                if let Some(dir) = projects_dialog()
                    .set_title("Open a project folder")
                    .pick_folder()
                {
                    state.actions.push(Action::OpenProject(dir));
                }
            }
            Action::OpenProject(root) => {
                let same = project::absolute(&root).ok();
                if state
                    .project
                    .as_ref()
                    .is_some_and(|p| Some(&p.root) == same.as_ref())
                {
                    return; // already open
                }
                if state.guard_unsaved(state.dirty_paths(), Action::OpenProject(root.clone())) {
                    return;
                }
                self.open_project(&root, true);
            }
            Action::CloseProject => {
                if state.guard_unsaved(state.dirty_paths(), Action::CloseProject) {
                    return;
                }
                self.close_project();
                self.state.settings.last_project = None;
            }
            Action::OpenFileDialog => {
                let mut dialog =
                    rfd::FileDialog::new().add_filter("HBC-2 source", &["has", "c", "h"]);
                if let Some(project) = &state.project {
                    dialog = dialog.set_directory(&project.root);
                }
                if let Some(path) = dialog.pick_file() {
                    state.actions.push(Action::OpenFile(path, None));
                }
            }
            Action::OpenFile(path, line) => self.open_file(&path, line),
            Action::NewFile(dir) => {
                let c = state
                    .project
                    .as_ref()
                    .is_some_and(|p| p.language == Language::C);
                state.dialog = Some(Dialog::NewFile {
                    dir,
                    name: if c { "newFile.c" } else { "newFile.has" }.into(),
                    error: None,
                })
            }
            Action::NewFolder(dir) => {
                state.dialog = Some(Dialog::NewFolder {
                    dir,
                    name: "lib".into(),
                    error: None,
                })
            }
            Action::RenameFile(path) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into();
                state.dialog = Some(Dialog::Rename {
                    path,
                    name,
                    error: None,
                });
            }
            Action::DeleteFile(path) => state.dialog = Some(Dialog::Delete { path }),
            Action::RefreshFiles => {
                if let Some(project) = &mut state.project {
                    project.rescan();
                }
            }
            Action::ShowFolder(path) => show_in_file_manager(&path),
            Action::MovePath(from, to_dir) => self.move_path(&from, &to_dir),
            Action::Save => {
                if let Some(path) = self.current_editor() {
                    self.state.save(&[path]);
                }
            }
            Action::SaveAll => {
                let paths = state.dirty_paths();
                state.save(&paths);
            }
            Action::CloseTab(path) => self.state.actions.push(Action::CloseTabs(vec![path])),
            Action::CloseTabs(paths) => {
                if state.guard_unsaved(paths.clone(), Action::CloseTabs(paths.clone())) {
                    return;
                }
                self.remove_editor_tabs(|p| paths.iter().any(|q| q == p));
                self.state.docs.retain(|d| !paths.contains(&d.path));
            }
            Action::CloseGroup(group) => {
                let paths = self.tab_group(group);
                if !paths.is_empty() {
                    self.state.actions.push(Action::CloseTabs(paths));
                }
            }
            Action::CloseCurrentTab => {
                if let Some(path) = self.current_editor() {
                    self.state.actions.push(Action::CloseTab(path));
                }
            }
            Action::Undo | Action::Redo => {
                // Files cannot be edited while the program runs.
                if self.state.snapshot.state == RunState::Running {
                    return;
                }
                if let Some(path) = self.active_editor() {
                    self.state.undo(ctx, &path, matches!(action, Action::Redo));
                }
            }
            Action::OpenFind(mode) => {
                let editor = self.active_editor();
                let state = &mut self.state;
                match mode {
                    FindMode::File if editor.is_none() => return,
                    FindMode::Files if state.project.is_none() => {
                        return state.console.error("Open a project first (File menu).");
                    }
                    _ => {}
                }
                // Search for the selected text, when it is on one line.
                if let Some(path) = &editor
                    && let Some(doc) = state.docs.iter().find(|d| &d.path == path)
                    && let Some(range) = egui::text_edit::TextEditState::load(ctx, editor_id(path))
                        .and_then(|s| s.cursor.char_range())
                {
                    let range = range.as_sorted_char_range();
                    let selected: String = doc
                        .text
                        .chars()
                        .skip(range.start.0)
                        .take(range.end.0 - range.start.0)
                        .collect();
                    if !selected.is_empty() && !selected.contains('\n') {
                        state.find.query = selected;
                    }
                }
                state.find.mode = Some(mode);
                state.find.focus_query = true;
                state.find.current = None;
            }
            Action::CloseFind => {
                let find = &mut state.find;
                find.mode = None;
                // Back in the editor, the current match is selected.
                if let Some((path, m)) = find.current.take()
                    && let Some(doc) = state.docs.iter_mut().find(|d| d.path == path)
                {
                    doc.select = Some(m);
                }
                if let Some(path) = self.active_editor()
                    && let Some(doc) = self.state.docs.iter_mut().find(|d| d.path == path)
                {
                    doc.focus = true;
                }
            }
            Action::ShowMatch {
                path,
                line,
                columns,
            } => {
                let path = project::absolute(&path).unwrap_or(path);
                self.open_file(&path, None);
                if let Some(doc) = self.state.docs.iter_mut().find(|d| d.path == path) {
                    let start = char_index_of_line(&doc.text, line);
                    doc.select = Some(start + columns.start..start + columns.end);
                    // The editor keeps its selection only while it has the keyboard.
                    doc.focus = true;
                }
            }
            Action::Assemble => {
                state.assemble();
            }
            Action::Run => {
                if state.prepare_to_run(Action::Run) {
                    state.emulator.run();
                }
            }
            Action::Pause => state.emulator.pause(),
            Action::Step => {
                if state.prepare_to_run(Action::Step) {
                    let pc = state.emulator.runner.sync().cpu.pc;
                    match state.c_line_addresses(pc) {
                        Some(addresses) => state.emulator.step_within(addresses),
                        None => state.emulator.step(1),
                    }
                }
            }
            Action::Reset => {
                state.emulator.reset();
                state.followed = None;
                state.console.info("The machine restarted.");
            }
            Action::ExportImage => state.export_image(),
            Action::LoadImage => state.load_image(),
            Action::SaveEeprom => state.save_eeprom(),
            Action::SetTarget(target) => {
                if state.project_settings.target != target {
                    state.project_settings.target = target;
                    state.edits += 1; // the loaded build no longer matches
                    let name = match target {
                        TargetSetting::Ram => "RAM",
                        TargetSetting::Eeprom => "EEPROM",
                    };
                    state
                        .console
                        .info(format!("Target: {name}. Assemble (F7) to rebuild."));
                }
            }
            Action::SetStrip(strip) => {
                if state.project_settings.strip != strip {
                    state.project_settings.strip = strip;
                    state.edits += 1; // the loaded build no longer matches
                    let what = if strip {
                        "Unused code is left out"
                    } else {
                        "Unused code is kept"
                    };
                    state
                        .console
                        .info(format!("{what}. Assemble (F7) to rebuild."));
                }
            }
            Action::SetPeripherals(peripherals) => {
                state.project_settings.peripherals = peripherals;
                state.emulator.peripherals = peripherals;
                let breakpoints = state.breakpoint_addresses();
                match state.emulator.rebuild(&breakpoints) {
                    Ok(()) => state
                        .console
                        .info("Peripherals changed: the machine restarted."),
                    Err(e) => state.console.error(e),
                }
            }
            Action::SetSpeed(speed) => {
                state.settings.speed = speed.into();
                state.emulator.runner.set_speed(speed);
            }
            Action::RtcInterrupt => state.emulator.runner.exec(|machine| {
                machine.rtc_interrupt();
            }),
            Action::BreakpointsChanged => {
                if state.emulator.program.is_some() {
                    let addresses = state.breakpoint_addresses();
                    state.emulator.set_breakpoints(&addresses);
                }
            }
            Action::ToggleAddressBreakpoint(address) => {
                if let Some((path, line)) = state.source_location(address) {
                    let lines = state.project_settings.breakpoints.entry(path).or_default();
                    if !lines.remove(&line) {
                        lines.insert(line);
                    }
                    state.actions.push(Action::BreakpointsChanged);
                } else {
                    let on = !state.snapshot.breakpoints.contains(address);
                    state.emulator.runner.set_breakpoint(address, on);
                }
            }
            Action::Quit => {
                state.quit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::ShowDialog(dialog) => state.dialog = Some(dialog),
        }
    }

    pub fn open_project(&mut self, root: &Path, restore_files: bool) {
        let project = match Project::open(root) {
            Ok(project) => project,
            Err(e) => {
                self.state.message("Cannot open the project", e.to_string());
                if self.state.settings.last_project.as_deref() == Some(root) {
                    self.state.settings.last_project = None;
                }
                return;
            }
        };
        self.close_project();
        let state = &mut self.state;
        let root = project.root.clone();
        state.settings.add_recent(&root);
        state.settings.last_project = Some(root.clone());
        state.project_settings = state
            .settings
            .projects
            .get(&root)
            .cloned()
            .unwrap_or_default();
        state.emulator.peripherals = state.project_settings.peripherals;
        let _ = state.emulator.rebuild(&[]);
        state
            .console
            .info(format!("Opened project {}.", root.display()));
        let mut files = if restore_files {
            state.project_settings.open_files.clone()
        } else {
            Vec::new()
        };
        if restore_files && files.is_empty() {
            files.push(project.entry());
        }
        state.project = Some(project);
        for path in files {
            if path.is_file() {
                self.open_file(&path, None);
            }
        }
    }

    fn close_project(&mut self) {
        self.store_project_settings();
        let state = &mut self.state;
        if state.project.take().is_none() {
            return;
        }
        state.docs.clear();
        state.build = None;
        state.building = None; // its result is not wanted any more
        state.loaded_edits = None;
        state.project_settings = ProjectSettings::default();
        state.emulator.peripherals = Peripherals::default();
        state.emulator.unload();
        state.followed = None;
        self.remove_editor_tabs(|_| true);
    }

    /// Removes the editor tabs whose path matches. The start page takes the
    /// place of the last one, so the editor group never goes away.
    fn remove_editor_tabs(&mut self, remove: impl Fn(&Path) -> bool) {
        let editor = |t: &Tab| matches!(t, Tab::Editor(p) if remove(p));
        let kept = self
            .dock
            .iter_all_tabs()
            .any(|(_, t)| matches!(t, Tab::Editor(_)) && !editor(t));
        if !kept
            && self.dock.find_tab(&Tab::Welcome).is_none()
            && let Some(path) = self.dock.find_tab_from(|t| matches!(t, Tab::Editor(_)))
            && let Ok(leaf) = self.dock.leaf_mut(path.node_path())
        {
            leaf.append_tab(Tab::Welcome);
        }
        self.dock.retain_tabs(|t| !editor(t));
    }

    /// Moves a file or folder into `dir`, on disk and in the open editors
    /// and breakpoints.
    fn move_path(&mut self, from: &Path, dir: &Path) {
        let state = &mut self.state;
        let Some(name) = from.file_name() else { return };
        let to = dir.join(name);
        let shown = |path: &Path| match &state.project {
            Some(project) => project.include_path(path),
            None => path.display().to_string(),
        };
        if to.exists() {
            let error = format!(
                "Cannot move {}: {} already exists.",
                shown(from),
                shown(&to)
            );
            return state.console.error(error);
        }
        if let Err(e) = std::fs::rename(from, &to) {
            let error = format!("Cannot move {}: {e}", shown(from));
            return state.console.error(error);
        }
        let info = format!(
            "Moved {} to {}. .include lines that use the old path must be updated by hand.",
            shown(from),
            shown(&to)
        );
        state.console.info(info);

        // A path inside what moved, at its new place.
        let moved = |path: &Path| {
            let rest = path.strip_prefix(from).ok()?;
            Some(if rest.as_os_str().is_empty() {
                to.clone()
            } else {
                to.join(rest)
            })
        };
        for doc in &mut state.docs {
            if let Some(new) = moved(&doc.path) {
                doc.set_path(new);
            }
        }
        for (_, tab) in self.dock.iter_all_tabs_mut() {
            if let Tab::Editor(path) = tab
                && let Some(new) = moved(path)
            {
                *path = new;
            }
        }
        let breakpoints = &mut state.project_settings.breakpoints;
        let old: Vec<PathBuf> = breakpoints
            .keys()
            .filter(|path| path.starts_with(from))
            .cloned()
            .collect();
        for path in old {
            if let (Some(lines), Some(new)) = (breakpoints.remove(&path), moved(&path)) {
                breakpoints.insert(new, lines);
            }
        }
        state.edits += 1;
        state.actions.push(Action::RefreshFiles);
    }

    pub fn open_file(&mut self, path: &Path, line: Option<u32>) {
        let path = match project::absolute(path) {
            Ok(path) => path,
            Err(e) => return self.state.console.error(format!("{}: {e}", path.display())),
        };
        if !self.state.docs.iter().any(|d| d.path == path) {
            match Document::open(&path) {
                Ok(doc) => self.state.docs.push(doc),
                Err(e) => {
                    return self
                        .state
                        .console
                        .error(format!("Cannot open {}: {e}", path.display()));
                }
            }
        }
        let tab = Tab::Editor(path.clone());
        if let Some(found) = self.dock.find_tab(&tab) {
            let _ = self.dock.set_active_tab(found);
            self.dock.set_focused_node_and_surface(found.node_path());
        } else {
            self.add_editor_tab(tab);
        }
        if let Some(doc) = self.state.docs.iter_mut().find(|d| d.path == path) {
            doc.scroll_to = line.or(doc.scroll_to);
            doc.scroll_gently = false;
        }
    }

    /// Puts a new editor tab next to the other editors, or in place of the
    /// start page.
    fn add_editor_tab(&mut self, tab: Tab) {
        let near = self
            .dock
            .find_tab_from(|t| matches!(t, Tab::Editor(_)))
            .or_else(|| self.dock.find_tab(&Tab::Welcome));
        match near.and_then(|path| self.dock.leaf_mut(path.node_path()).ok()) {
            Some(leaf) => leaf.append_tab(tab.clone()),
            None => self.dock.push_to_first_leaf(tab.clone()),
        }
        // The editor group holds the new tab, so it stays when the start page goes.
        if let Some(welcome) = self.dock.find_tab(&Tab::Welcome) {
            self.dock.remove_tab(welcome);
        }
        if let Some(found) = self.dock.find_tab(&tab) {
            let _ = self.dock.set_active_tab(found);
            self.dock.set_focused_node_and_surface(found.node_path());
        }
    }

    /// Path of the editor tab shown in the editor group, focused or not.
    fn active_editor(&self) -> Option<PathBuf> {
        let path = self.dock.find_tab_from(|t| matches!(t, Tab::Editor(_)))?;
        let leaf = self.dock.leaf(path.node_path()).ok()?;
        match leaf.tabs.get(leaf.active.0) {
            Some(Tab::Editor(path)) => Some(path.clone()),
            _ => None,
        }
    }

    /// The open editors in `group`, in tab order.
    fn tab_group(&mut self, group: TabGroup) -> Vec<PathBuf> {
        let editors: Vec<PathBuf> = self
            .dock
            .iter_all_tabs()
            .filter_map(|(_, t)| match t {
                Tab::Editor(path) => Some(path.clone()),
                _ => None,
            })
            .collect();
        match group {
            TabGroup::ToTheRight(path) => match editors.iter().position(|p| *p == path) {
                Some(i) => editors[i + 1..].to_vec(),
                None => Vec::new(),
            },
            TabGroup::Saved => editors
                .into_iter()
                .filter(|p| !self.state.docs.iter().any(|d| &d.path == p && d.is_dirty()))
                .collect(),
            TabGroup::Others(path) => {
                let Some(keep) = path.or_else(|| self.current_editor()) else {
                    return Vec::new();
                };
                editors.into_iter().filter(|p| *p != keep).collect()
            }
            TabGroup::All => editors,
        }
    }

    /// Path of the editor tab in the focused group, if it is an editor.
    fn current_editor(&mut self) -> Option<PathBuf> {
        match self.dock.find_active_focused() {
            Some((_, Tab::Editor(path))) => Some(path.clone()),
            _ => None,
        }
    }
}

impl State {
    pub fn dirty_paths(&self) -> Vec<PathBuf> {
        self.docs
            .iter()
            .filter(|d| d.is_dirty())
            .map(|d| d.path.clone())
            .collect()
    }

    /// If any of `paths` has unsaved changes, asks what to do and returns
    /// true: `then` runs once they are saved or discarded.
    fn guard_unsaved(&mut self, paths: Vec<PathBuf>, then: Action) -> bool {
        let dirty: Vec<PathBuf> = paths
            .into_iter()
            .filter(|p| self.docs.iter().any(|d| &d.path == p && d.is_dirty()))
            .collect();
        if dirty.is_empty() {
            return false;
        }
        self.dialog = Some(Dialog::Unsaved {
            paths: dirty,
            then: Box::new(then),
        });
        true
    }

    /// Saves these documents. Returns false if one could not be saved.
    pub fn save(&mut self, paths: &[PathBuf]) -> bool {
        let mut ok = true;
        for doc in self.docs.iter_mut().filter(|d| paths.contains(&d.path)) {
            if let Err(e) = doc.save() {
                ok = false;
                self.console
                    .error(format!("Cannot save {}: {e}", doc.path.display()));
            }
        }
        if let Some(project) = &mut self.project {
            project.rescan();
        }
        ok
    }

    /// Undoes (or redoes) the last edit of a file, with the undo history of
    /// its editor, then gives the editor the keyboard.
    fn undo(&mut self, ctx: &egui::Context, path: &Path, redo: bool) {
        let id = editor_id(path);
        let Some(doc) = self.docs.iter_mut().find(|d| d.path == path) else {
            return;
        };
        let mut editor = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
        let mut history = editor.undoer();
        if let Some((range, text)) = history_step(&mut history, &editor, &doc.text, redo) {
            let before = std::mem::replace(&mut doc.text, text);
            // The cursor goes back exactly as it was: the editor drops the
            // redo steps as soon as it differs from the history's.
            editor.cursor.set_char_range(Some(range));
            let sorted = range.as_sorted_char_range();
            doc.select = Some(sorted.start.0..sorted.end.0); // brings it into view
            self.edits += 1;
            let lines = self
                .project_settings
                .breakpoints
                .entry(path.to_path_buf())
                .or_default();
            let old = lines.clone();
            shift_breakpoints(lines, &before, &doc.text);
            if *lines != old {
                self.actions.push(Action::BreakpointsChanged);
            }
        }
        editor.set_undoer(history);
        editor.store(ctx, id);
        // The editor is the tab shown, drawn in this frame: it can take the
        // keyboard now (before it is drawn again, which would drop its selection).
        ctx.memory_mut(|m| m.request_focus(id));
    }

    pub fn discard(&mut self, paths: &[PathBuf]) {
        for doc in self.docs.iter_mut().filter(|d| paths.contains(&d.path)) {
            doc.revert();
        }
        self.edits += 1;
    }

    pub fn message(&mut self, title: &str, text: String) {
        self.console.error(format!("{title}: {text}"));
        self.dialog = Some(Dialog::Message {
            title: title.to_string(),
            text,
        });
    }

    /// Source file and line of the instruction at a RAM address.
    pub fn source_location(&self, address: u16) -> Option<(PathBuf, u32)> {
        let program = self.emulator.program.as_ref()?;
        let line = program.source_of(address as u32)?;
        Some((program.files.get(line.file)?.clone(), line.line))
    }

    /// Whether the editor, disassembly and memory views follow the PC: when paused
    /// (after each step), and while running slowly enough to watch.
    pub fn follows_execution(&self) -> bool {
        self.snapshot.state == RunState::Paused
            || matches!(Speed::from(self.settings.speed), Speed::Hz(hz) if hz <= FOLLOW_MAX_HZ)
    }

    /// Line of the next instruction in `path`, when paused or running slowly.
    pub fn pc_line(&self, path: &Path) -> Option<u32> {
        if !self.follows_execution() {
            return None;
        }
        let (file, line) = self.source_location(self.snapshot.cpu.pc)?;
        (file == path).then_some(line)
    }

    /// Diagnostics of the last build for one file: (line, is_error, message).
    pub fn diagnostics_for(&self, path: &Path) -> Vec<(u32, bool, String)> {
        let Some(build) = &self.build else {
            return Vec::new();
        };
        build
            .diagnostics
            .iter()
            .filter(|d| d.file.as_deref() == Some(path) && d.line > 0)
            .map(|d| (d.line, d.severity == Severity::Error, d.message.clone()))
            .collect()
    }

    /// RAM addresses of the breakpoint lines, for the loaded program.
    pub fn breakpoint_addresses(&self) -> Vec<u16> {
        match &self.emulator.program {
            Some(program) => breakpoint_addresses(program, &self.project_settings),
            None => Vec::new(),
        }
    }

    pub fn is_stale(&self) -> bool {
        self.loaded_edits.is_some_and(|e| e != self.edits)
    }

    /// Before Run or Step (`then`): assembles first if nothing is loaded, or
    /// if the sources changed and the program has not started yet. Returns
    /// false if there is nothing (valid) to run now. A C project builds in
    /// the background: `then` runs once its build is loaded.
    fn prepare_to_run(&mut self, then: Action) -> bool {
        if let Some(pending) = &mut self.building {
            pending.then = Some(then);
            return false;
        }
        let fresh_machine = self.snapshot.ticks == 0;
        let needs_build = self.project.is_some()
            && (self.emulator.image().is_none() || (self.is_stale() && fresh_machine));
        if needs_build && !self.assemble() {
            if let Some(pending) = &mut self.building {
                pending.then = Some(then);
            }
            return false;
        }
        if self.emulator.image().is_none() {
            self.console.error(
                "Nothing to run: open a project and assemble it, or load an image (Emulator menu).",
            );
            return false;
        }
        true
    }

    /// The program's target: always RAM for a C project (its startup code
    /// runs from RAM).
    pub fn target(&self) -> Target {
        match &self.project {
            Some(p) if p.language == Language::C => Target::Ram,
            _ => self.project_settings.target.into(),
        }
    }

    /// Whether `path` is shown but not edited: the compiled files of a C
    /// project and the compiler's own files (startup code, runtime,
    /// library), which the debugger opens.
    pub fn is_generated(&self, path: &Path) -> bool {
        let compiler = normalize(&hbc2_cc::compiler_dir());
        self.project
            .as_ref()
            .is_some_and(|p| path.starts_with(p.build_dir()))
            || path.starts_with(compiler)
    }

    /// A C build runs in the background.
    pub fn is_building(&self) -> bool {
        self.building.is_some()
    }

    /// Assembles the project and loads it, paused at power-on. A C project
    /// is saved, then compiled and assembled in the background (see
    /// [`State::poll_build`]): returns false, as nothing is loaded yet.
    pub fn assemble(&mut self) -> bool {
        let Some(project) = &self.project else {
            self.console
                .error("Open or create a project first (File menu).");
            return false;
        };
        if self.building.is_some() {
            self.console.info("A build is already in progress.");
            return false;
        }
        let name = project.name();
        if project.language == Language::C {
            let (sources, dir) = (project.c_sources(), project.build_dir());
            if !self.save(&self.dirty_paths()) {
                return false;
            }
            self.console.info(format!(
                "Building {name}… (in progress, the IDE stays usable)"
            ));
            let (send, result) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = send.send(build_c(&sources, &dir));
            });
            self.building = Some(PendingBuild {
                result,
                started: Instant::now(),
                edits: self.edits,
                then: None,
            });
            return false;
        }
        let target = self.target();
        let entry = project.entry();
        let sources = Sources(
            self.docs
                .iter()
                .map(|d| (d.path.clone(), d.text.as_str()))
                .collect(),
        );
        let target_name = match target {
            Target::Ram => "RAM",
            Target::Eeprom => "EEPROM",
        };
        self.console
            .info(format!("Assembling {name} for {target_name}…"));
        let started = Instant::now();
        let options = hbc2_asm::Options {
            target,
            strip: self.project_settings.strip,
        };
        let result = hbc2_asm::assemble_with(&entry, &sources, options).map(|p| {
            let warnings = p.warnings.clone();
            (p, warnings)
        });
        self.finish_build(result, false, started, self.edits)
    }

    /// Once a frame: takes the result of a background build when it is
    /// ready, loads it, then runs what was waiting for it (Run, Step).
    pub fn poll_build(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.building else {
            return;
        };
        match pending.result.try_recv() {
            Ok(result) => {
                let pending = self.building.take().expect("a build is pending");
                let ok = self.finish_build(result, true, pending.started, pending.edits);
                if ok && let Some(then) = pending.then {
                    self.actions.push(then);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.building = None;
                self.console.error("The build stopped unexpectedly.");
            }
        }
    }

    /// Shows the result of an assembly or a C build (`c`), and loads the
    /// program. `edits` is the edit count of the sources it was made from.
    fn finish_build(&mut self, result: BuildResult, c: bool, started: Instant, edits: u64) -> bool {
        let elapsed = started.elapsed();
        let target = self.target();
        match result {
            Ok((program, warnings)) => {
                for warning in &warnings {
                    self.push_diagnostic(warning);
                }
                self.console.push(
                    Level::Success,
                    format!(
                        "{} in {:.0} ms: {} bytes used{}, {} warning(s). Loaded, paused at power-on.",
                        if c { "Built" } else { "Assembled" },
                        elapsed.as_secs_f64() * 1000.0,
                        program.bytes_used,
                        match program.bytes_removed() {
                            0 => String::new(),
                            n => format!(" ({n} unused bytes left out)"),
                        },
                        warnings.len()
                    ),
                    None,
                );
                let mut code_lines: HashMap<PathBuf, BTreeSet<u32>> = HashMap::new();
                for line in &program.lines {
                    if let Some(file) = program.files.get(line.file) {
                        code_lines
                            .entry(file.clone())
                            .or_default()
                            .insert(line.line);
                    }
                }
                let breakpoints = breakpoint_addresses(&program, &self.project_settings);
                let image = match target {
                    Target::Ram => Image::Ram(program.image.clone().into()),
                    Target::Eeprom => Image::Eeprom(program.image.clone().into()),
                };
                self.build = Some(Build {
                    ok: true,
                    diagnostics: warnings,
                    code_lines,
                });
                if let Err(e) = self
                    .emulator
                    .load(image, Some(Arc::new(program)), &breakpoints)
                {
                    self.console.error(e);
                    return false;
                }
                self.loaded_edits = Some(edits);
                self.followed = None;
                true
            }
            Err(errors) => {
                for error in &errors {
                    self.push_diagnostic(error);
                }
                let count = errors
                    .iter()
                    .filter(|e| e.severity == Severity::Error)
                    .count();
                let what = if c { "Build" } else { "Assembly" };
                self.console
                    .error(format!("{what} failed: {count} error(s)."));
                self.build = Some(Build {
                    ok: false,
                    diagnostics: errors,
                    code_lines: HashMap::new(),
                });
                false
            }
        }
    }

    /// F10: in a C file, the addresses of the instructions of the line the
    /// CPU is on (at `pc`), so that the whole line runs; `None` elsewhere
    /// (one instruction). The line stops at the first instruction of
    /// another line or of an assembly file (a runtime routine, an
    /// interrupt handler).
    fn c_line_addresses(&self, pc: u16) -> Option<Vec<u16>> {
        let program = self.emulator.program.as_ref()?;
        let (path, line) = self.source_location(pc)?;
        if crate::highlight::Syntax::of(&path) != crate::highlight::Syntax::C {
            return None;
        }
        let file = program.files.iter().position(|f| *f == path)?;
        Some(
            program
                .lines
                .iter()
                .filter(|l| l.file == file && l.line == line)
                .filter_map(|l| ram_address(program.target, l.address))
                .collect(),
        )
    }

    fn push_diagnostic(&mut self, d: &Diagnostic) {
        let level = match d.severity {
            Severity::Error => Level::Error,
            Severity::Warning => Level::Warning,
        };
        let location = d.file.clone().filter(|_| d.line > 0).map(|f| (f, d.line));
        self.console.push(level, d.to_string(), location);
    }

    fn export_image(&mut self) {
        let Some(image) = self.emulator.image().cloned() else {
            self.console.error("Nothing to export: assemble first.");
            return;
        };
        let (bytes, name) = match &image {
            Image::Ram(bytes) => (bytes.clone(), "ram.bin"),
            Image::Eeprom(bytes) => (bytes.clone(), "eeprom.bin"),
        };
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Binary image", &["bin"])
            .set_file_name(name);
        if let Some(project) = &self.project {
            dialog = dialog.set_directory(&project.root);
        }
        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, &bytes[..]) {
                Ok(()) => self
                    .console
                    .success(format!("Image written to {}.", path.display())),
                Err(e) => self
                    .console
                    .error(format!("Cannot write {}: {e}", path.display())),
            }
        }
    }

    fn load_image(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Load a RAM (64 KiB) or EEPROM (1 MiB) image")
            .add_filter("Binary image", &["bin", "rom"])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(Image::from_bytes)
            .and_then(|image| self.emulator.load(image, None, &[]));
        match result {
            Ok(()) => {
                self.loaded_edits = None;
                self.followed = None;
                self.console
                    .success(format!("Loaded {} (no debug information).", path.display()));
            }
            Err(e) => self.message("Cannot load the image", e),
        }
    }

    fn save_eeprom(&mut self) {
        let Some(eeprom) = self.snapshot.eeprom.clone() else {
            self.console
                .error("No EEPROM plugged in (assemble for the EEPROM target).");
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Binary image", &["bin"])
            .set_file_name("eeprom.bin")
            .save_file()
        {
            match std::fs::write(&path, eeprom.memory()) {
                Ok(()) => self
                    .console
                    .success(format!("EEPROM content written to {}.", path.display())),
                Err(e) => self
                    .console
                    .error(format!("Cannot write {}: {e}", path.display())),
            }
        }
    }
}

/// Whether the editor of `path` has an edit to undo (or redo).
pub fn can_undo(docs: &[Document], ctx: &egui::Context, path: &Path, redo: bool) -> bool {
    let Some(doc) = docs.iter().find(|d| d.path == path) else {
        return false;
    };
    let editor = egui::text_edit::TextEditState::load(ctx, editor_id(path)).unwrap_or_default();
    history_step(&mut editor.undoer(), &editor, &doc.text, redo).is_some()
}

/// An editor's undo history: (cursor, text) states.
type History = egui::util::undoer::Undoer<(egui::text::CCursorRange, String)>;

/// Moves `history` one undo (or redo) step back (or forward), to the first
/// state with another text than `text`, and returns that state. Steps that
/// only moved the cursor are skipped: they would change nothing visible.
fn history_step(
    history: &mut History,
    editor: &egui::text_edit::TextEditState,
    text: &str,
    redo: bool,
) -> Option<(egui::text::CCursorRange, String)> {
    use egui::text::{CCursor, CCursorRange};
    // The history's latest state, read by undoing from a state it cannot
    // hold (undo then returns the latest state, and leaves it in place).
    let impossible = (CCursorRange::one(CCursor::new(usize::MAX)), String::new());
    let latest = history.clone().undo(&impossible).cloned();
    // An editor without the keyboard has dropped its selection: when the
    // text is the latest state's, so is the cursor (the history compares both).
    let mut current = match latest {
        Some(state) if state.1 == text => state,
        _ => {
            let cursor = editor
                .cursor
                .char_range()
                .unwrap_or_else(|| CCursorRange::one(CCursor::new(0)));
            (cursor, text.to_string())
        }
    };
    loop {
        let state = if redo {
            history.redo(&current)
        } else {
            history.undo(&current)
        }?
        .clone();
        if state.1 != text {
            return Some(state);
        }
        current = state;
    }
}

/// Compiles and assembles a C project: the program and its warnings, or the
/// errors (with the warnings). The library is compiled into `dir/lib` when
/// it changed.
fn build_c(sources: &[PathBuf], dir: &Path) -> BuildResult {
    let library = hbc2_cc::build_library(&dir.join("lib"))?;
    let build = hbc2_cc::build_program(sources, &library, dir)?;
    Ok((build.program, build.warnings))
}

fn breakpoint_addresses(program: &Program, settings: &ProjectSettings) -> Vec<u16> {
    let mut addresses = Vec::new();
    for (path, lines) in &settings.breakpoints {
        let Some(file) = program.files.iter().position(|f| f == path) else {
            continue;
        };
        for &line in lines {
            if let Some(address) = program.address_of(file, line)
                && let Some(address) = ram_address(program.target, address)
            {
                addresses.push(address);
            }
        }
    }
    addresses
}

/// A folder dialog that opens in `Documents/hbc2_projects` when it exists.
fn projects_dialog() -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new();
    match project::projects_dir() {
        Some(dir) if dir.is_dir() => dialog.set_directory(dir),
        _ => dialog,
    }
}

/// Opens the system's file manager on `path`: a folder is opened, a file is
/// shown selected in its folder (on Windows; elsewhere its folder opens).
fn show_in_file_manager(path: &Path) {
    #[cfg(windows)]
    if path.is_file() {
        use std::os::windows::process::CommandExt;
        // Explorer wants the quotes around the path only, not around "/select,".
        let _ = std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn();
        return;
    }
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    // Launch the file manager directly: a file:// URL would open the browser.
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program).arg(dir).spawn();
}
