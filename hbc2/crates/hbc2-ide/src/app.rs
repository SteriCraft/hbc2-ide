//! The IDE's state and everything it can do.
//!
//! Views and menus never change the state directly when it involves more
//! than their own widget: they queue an [`Action`], and the actions run once
//! the frame's UI is built. This keeps borrowing simple (a view only sees
//! what it shows) and gives one place where each operation is written.

use std::collections::{BTreeSet, HashMap};
use std::io;
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
use crate::document::Document;
use crate::emulator::{Emulator, Image, Peripherals, ram_address};
use crate::project::{self, Project};
use crate::settings::{ProjectSettings, Settings, TargetSetting};
use crate::tabs::Tab;
use crate::views::disasm::DisasmView;
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
    /// default interrupt handler.
    CreateProject(PathBuf, bool),
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
    ShowFolder,
    Save,
    SaveAll,
    /// Closes an editor tab, asking about unsaved changes.
    CloseTab(PathBuf),
    CloseCurrentTab,
    Assemble,
    Run,
    Pause,
    Step,
    Reset,
    ExportImage,
    LoadImage,
    SaveEeprom,
    SetTarget(TargetSetting),
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
            state: State {
                settings,
                project: None,
                project_settings: ProjectSettings::default(),
                docs: Vec::new(),
                console: Console::default(),
                emulator,
                build: None,
                edits: 0,
                loaded_edits: None,
                snapshot,
                dialog: None,
                actions: Vec::new(),
                monitor_view: MonitorView::default(),
                memory_view: MemoryView::default(),
                disasm_view: DisasmView::default(),
                followed: None,
                reported_stop: None,
                last_disk_check: 0.0,
                quit_confirmed: false,
                title: String::new(),
                tools_min_width: 0.0,
            },
        };
        ide.state.disasm_view.follow_pc = ide.state.settings.follow_pc;
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
        self.shortcuts(&ctx);
        self.watch_emulator();
        self.check_disk(&ctx);
        self.handle_close_request(&ctx);

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
            (K::new(M::COMMAND, Key::W), Action::CloseCurrentTab),
            (K::new(M::COMMAND, Key::O), Action::OpenProjectDialog),
        ];
        for (shortcut, action) in table {
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                self.state.actions.push(action);
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
        if s.state == RunState::Paused && s.ticks > 0 && self.state.settings.follow_pc {
            let key = (s.ticks, s.cpu.pc);
            if self.state.followed != Some(key) {
                self.state.followed = Some(key);
                if let Some((path, line)) = self.state.source_location(s.cpu.pc) {
                    self.state.actions.push(Action::OpenFile(path, Some(line)));
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
                    interrupt_handler: true,
                    error: None,
                });
            }
            Action::CreateProject(dir, interrupt_handler) => {
                match Project::create(&dir, interrupt_handler) {
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
                let mut dialog = rfd::FileDialog::new().add_filter("HBC-2 source", &["has"]);
                if let Some(project) = &state.project {
                    dialog = dialog.set_directory(&project.root);
                }
                if let Some(path) = dialog.pick_file() {
                    state.actions.push(Action::OpenFile(path, None));
                }
            }
            Action::OpenFile(path, line) => self.open_file(&path, line),
            Action::NewFile(dir) => {
                state.dialog = Some(Dialog::NewFile {
                    dir,
                    name: "newFile.has".into(),
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
            Action::ShowFolder => {
                if let Some(project) = &state.project {
                    ctx.open_url(egui::OpenUrl::new_tab(format!(
                        "file:///{}",
                        project.root.display()
                    )));
                }
            }
            Action::Save => {
                if let Some(path) = self.current_editor() {
                    self.state.save(&[path]);
                }
            }
            Action::SaveAll => {
                let paths = state.dirty_paths();
                state.save(&paths);
            }
            Action::CloseTab(path) => {
                let dirty = state.docs.iter().any(|d| d.path == path && d.is_dirty());
                if dirty && state.guard_unsaved(vec![path.clone()], Action::CloseTab(path.clone()))
                {
                    return;
                }
                self.remove_editor_tabs(|p| p == path);
                self.state.docs.retain(|d| d.path != path);
            }
            Action::CloseCurrentTab => {
                if let Some(path) = self.current_editor() {
                    self.state.actions.push(Action::CloseTab(path));
                }
            }
            Action::Assemble => {
                state.assemble();
            }
            Action::Run => {
                if state.prepare_to_run() {
                    state.emulator.run();
                }
            }
            Action::Pause => state.emulator.pause(),
            Action::Step => {
                if state.prepare_to_run() {
                    state.emulator.step(1);
                }
            }
            Action::Reset => {
                state.emulator.reset();
                state.followed = None;
                state.console.info("Power cycle: the machine restarted.");
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

    /// Line of the next instruction in `path`, when paused.
    pub fn pc_line(&self, path: &Path) -> Option<u32> {
        if self.snapshot.state != RunState::Paused {
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

    /// Before Run or Step: assembles first if nothing is loaded, or if the
    /// sources changed and the program has not started yet. Returns false
    /// if there is nothing (valid) to run.
    fn prepare_to_run(&mut self) -> bool {
        let fresh_machine = self.snapshot.ticks == 0;
        let needs_build = self.project.is_some()
            && (self.emulator.image().is_none() || (self.is_stale() && fresh_machine));
        if needs_build && !self.assemble() {
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

    /// Assembles the project and loads it, paused at power-on.
    pub fn assemble(&mut self) -> bool {
        let Some(project) = &self.project else {
            self.console
                .error("Open or create a project first (File menu).");
            return false;
        };
        let target: Target = self.project_settings.target.into();
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
            .info(format!("Assembling {} for {target_name}…", project.name()));
        let started = Instant::now();
        let result = hbc2_asm::assemble(&entry, &sources, target);
        let elapsed = started.elapsed();

        match result {
            Ok(program) => {
                for warning in &program.warnings {
                    self.push_diagnostic(warning);
                }
                self.console.push(
                    Level::Success,
                    format!(
                        "Assembled in {:.0} ms: {} bytes used, {} warning(s). Loaded, paused at power-on.",
                        elapsed.as_secs_f64() * 1000.0,
                        program.bytes_used,
                        program.warnings.len()
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
                    diagnostics: program.warnings.clone(),
                    code_lines,
                });
                if let Err(e) = self
                    .emulator
                    .load(image, Some(Arc::new(program)), &breakpoints)
                {
                    self.console.error(e);
                    return false;
                }
                self.loaded_edits = Some(self.edits);
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
                self.console
                    .error(format!("Assembly failed: {count} error(s)."));
                self.build = Some(Build {
                    ok: false,
                    diagnostics: errors,
                    code_lines: HashMap::new(),
                });
                false
            }
        }
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
