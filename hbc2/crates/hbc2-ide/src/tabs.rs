//! The dockable tabs.

use std::path::PathBuf;

use eframe::egui::{self, RichText, WidgetText};
use egui_dock::tab_viewer::OnCloseResponse;
use hbc2_runner::RunState;
use serde::{Deserialize, Serialize};

use crate::app::{Action, State};
use crate::views;
use crate::views::editor::{Marks, editor_ui};
use crate::views::files::FileAction;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Welcome,
    Project,
    Editor(PathBuf),
    Console,
    Monitor,
    Cpu,
    Memory,
    Disassembly,
    Io,
}

impl Tab {
    /// Tool tabs, in the order of the View menu.
    pub const TOOLS: [Tab; 7] = [
        Tab::Project,
        Tab::Console,
        Tab::Monitor,
        Tab::Cpu,
        Tab::Disassembly,
        Tab::Memory,
        Tab::Io,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Tab::Welcome => "Start",
            Tab::Project => "Project",
            Tab::Editor(_) => "Editor",
            Tab::Console => "Console",
            Tab::Monitor => "Monitor",
            Tab::Cpu => "CPU",
            Tab::Memory => "Memory",
            Tab::Disassembly => "Disassembly",
            Tab::Io => "I/O",
        }
    }
}

pub struct Viewer<'a> {
    pub state: &'a mut State,
}

impl egui_dock::TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("tab", &*tab))
    }

    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        match tab {
            Tab::Editor(path) => {
                let doc = self.state.docs.iter().find(|d| &d.path == path);
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let dirty = doc.is_some_and(|d| d.is_dirty());
                format!("{name}{}", if dirty { " •" } else { "" }).into()
            }
            other => other.name().into(),
        }
    }

    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        if let Tab::Editor(path) = tab {
            response.clone().on_hover_text(path.display().to_string());
        }
    }

    fn on_close(&mut self, tab: &mut Tab) -> OnCloseResponse {
        let Tab::Editor(path) = tab else {
            return OnCloseResponse::Close;
        };
        if self
            .state
            .docs
            .iter()
            .any(|d| &d.path == path && d.is_dirty())
        {
            self.state.actions.push(Action::CloseTab(path.clone()));
            OnCloseResponse::Ignore
        } else {
            self.state.docs.retain(|d| &d.path != path);
            OnCloseResponse::Close
        }
    }

    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        // These views scroll by themselves.
        match tab {
            Tab::Welcome => [true, true],
            _ => [false, false],
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        let state = &mut *self.state;
        let snapshot = state.snapshot.clone();
        let baseline = state.emulator.baseline.clone();
        match tab {
            Tab::Welcome => crate::chrome::welcome(ui, state),
            Tab::Project => {
                let docs = &state.docs;
                let action = views::files::files_ui(
                    ui,
                    state.project.as_ref(),
                    &|p| docs.iter().any(|d| d.path == p),
                    &|p| docs.iter().any(|d| d.path == p && d.is_dirty()),
                );
                if let Some(action) = action {
                    state.actions.push(match action {
                        FileAction::Open(path) => Action::OpenFile(path, None),
                        FileAction::NewFile(dir) => Action::NewFile(dir),
                        FileAction::Rename(path) => Action::RenameFile(path),
                        FileAction::Delete(path) => Action::DeleteFile(path),
                        FileAction::Refresh => Action::RefreshFiles,
                        FileAction::ShowFolder => Action::ShowFolder,
                    });
                }
            }
            Tab::Editor(path) => editor_tab(ui, state, path),
            Tab::Console => {
                if let Some((path, line)) = state.console.ui(ui) {
                    state.actions.push(Action::OpenFile(path, Some(line)));
                }
            }
            Tab::Monitor => {
                let keyboard = snapshot.keyboard.is_some();
                state.monitor_view.ui(
                    ui,
                    snapshot.monitor.as_ref(),
                    keyboard,
                    &state.emulator.runner,
                );
            }
            Tab::Cpu => {
                let baseline = baseline
                    .as_deref()
                    .filter(|_| snapshot.state == RunState::Paused);
                views::cpu::cpu_ui(ui, &snapshot, baseline, state.emulator.program.as_deref());
            }
            Tab::Memory => {
                let baseline = baseline
                    .as_deref()
                    .filter(|_| snapshot.state == RunState::Paused);
                state
                    .memory_view
                    .ui(ui, &snapshot, baseline, &state.emulator.runner);
            }
            Tab::Disassembly => {
                let program = state.emulator.program.clone();
                if let Some(address) = state.disasm_view.ui(ui, &snapshot, program.as_ref()) {
                    state.actions.push(Action::ToggleAddressBreakpoint(address));
                }
            }
            Tab::Io => views::io::io_ui(ui, &snapshot),
        }
    }
}

fn editor_tab(ui: &mut egui::Ui, state: &mut State, path: &PathBuf) {
    let pc_line = state.pc_line(path);
    let diagnostics = state.diagnostics_for(path);
    let Some(index) = state.docs.iter().position(|d| &d.path == path) else {
        ui.label(format!("{} is not loaded.", path.display()));
        return;
    };

    if state.docs[index].conflict {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⚠ This file changed on disk.").color(ui.visuals().warn_fg_color),
            );
            if ui.button("Reload from disk").clicked() {
                let doc = &mut state.docs[index];
                match crate::document::Document::open(&doc.path) {
                    Ok(fresh) => {
                        *doc = fresh;
                        state.edits += 1;
                    }
                    Err(e) => state.console.error(format!("Cannot reload: {e}")),
                }
            }
            if ui.button("Keep my version").clicked() {
                state.docs[index].conflict = false;
            }
        });
    }

    let code_lines = state
        .build
        .as_ref()
        .and_then(|b| b.ok.then(|| b.code_lines.get(path)).flatten());
    let breakpoints = state
        .project_settings
        .breakpoints
        .entry(path.clone())
        .or_default();
    let response = editor_ui(
        ui,
        &mut state.docs[index],
        Marks {
            breakpoints,
            pc_line,
            diagnostics: &diagnostics,
            code_lines: if state.build.as_ref().is_some_and(|b| b.ok) {
                Some(code_lines.unwrap_or(&EMPTY))
            } else {
                None
            },
            font_size: state.settings.editor_font_size,
        },
    );
    if response.changed {
        state.edits += 1;
    }
    if response.breakpoints_changed {
        state.actions.push(Action::BreakpointsChanged);
    }
}

static EMPTY: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
