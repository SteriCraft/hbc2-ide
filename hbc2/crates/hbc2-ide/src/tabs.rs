//! The dockable tabs.

use std::path::PathBuf;

use eframe::egui::{self, RichText, WidgetText};
use egui_dock::TabStyle;
use egui_dock::tab_viewer::OnCloseResponse;
use hbc2_runner::RunState;
use serde::{Deserialize, Serialize};

use crate::app::{Action, State, TOOL_TABS, TabGroup};
use crate::find::FindMode;
use crate::views;
use crate::views::editor::{Marks, editor_ui};
use crate::views::files::FileAction;
use crate::views::find::find_bar;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Welcome,
    Project,
    Editor(PathBuf),
    Console,
    /// The CPU, then the IOD and peripherals.
    Cpu,
    Memory,
    Disassembly,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Welcome => "Start",
            Tab::Project => "Project",
            Tab::Editor(_) => "Editor",
            Tab::Console => "Console",
            Tab::Cpu => "CPU & I/O",
            Tab::Memory => "Memory",
            Tab::Disassembly => "Disassembly",
        }
    }
}

pub struct Viewer<'a> {
    pub state: &'a mut State,
    /// Width of the right-hand group, shared equally by its tabs.
    pub tools_width: f32,
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
        let Tab::Editor(path) = tab else { return };
        response.clone().on_hover_text(path.display().to_string());
        let actions = &mut self.state.actions;
        response.context_menu(|ui| {
            let item = |text: &str, shortcut: &str| {
                egui::Button::new(text.to_string()).shortcut_text(shortcut.to_string())
            };
            let mut clicked = |ui: &mut egui::Ui, button: egui::Button, action: Action| {
                if ui.add(button).clicked() {
                    actions.push(action);
                    ui.close();
                }
            };
            clicked(
                ui,
                item("Close this tab", "Ctrl+W"),
                Action::CloseTab(path.clone()),
            );
            clicked(
                ui,
                item("Close tabs to the right", ""),
                Action::CloseGroup(TabGroup::ToTheRight(path.clone())),
            );
            clicked(
                ui,
                item("Close saved files", ""),
                Action::CloseGroup(TabGroup::Saved),
            );
            clicked(
                ui,
                item("Close other tabs", "Ctrl+W+R"),
                Action::CloseGroup(TabGroup::Others(Some(path.clone()))),
            );
            clicked(
                ui,
                item("Close all tabs", "Ctrl+W+K"),
                Action::CloseGroup(TabGroup::All),
            );
        });
    }

    /// Only editor tabs close; the other tabs are fixed.
    fn is_closeable(&self, tab: &Tab) -> bool {
        matches!(tab, Tab::Editor(_))
    }

    /// The close action asks about unsaved changes and puts the start page
    /// back after the last editor.
    fn on_close(&mut self, tab: &mut Tab) -> OnCloseResponse {
        if let Tab::Editor(path) = tab {
            self.state.actions.push(Action::CloseTab(path.clone()));
        }
        OnCloseResponse::Ignore
    }

    fn allowed_in_windows(&self, _tab: &mut Tab) -> bool {
        false
    }

    fn tab_style_override(&self, tab: &Tab, global_style: &TabStyle) -> Option<TabStyle> {
        if !TOOL_TABS.contains(tab) || self.tools_width <= 0.0 {
            return None;
        }
        Some(TabStyle {
            minimum_width: Some((self.tools_width / TOOL_TABS.len() as f32).floor()),
            ..global_style.clone()
        })
    }

    fn scroll_bars(&self, _tab: &Tab) -> [bool; 2] {
        // Every view scrolls by itself.
        [false, false]
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        let state = &mut *self.state;
        let snapshot = state.snapshot.clone();
        let baseline = state.emulator.baseline.clone();
        match tab {
            Tab::Welcome => {
                // "Find in files" also works with no file open.
                if state.find.mode == Some(FindMode::Files) {
                    find_bar(ui, state, None);
                }
                egui::ScrollArea::both()
                    .id_salt("welcome")
                    .auto_shrink(false)
                    .show(ui, |ui| crate::chrome::welcome(ui, state));
            }
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
                        FileAction::NewFolder(dir) => Action::NewFolder(dir),
                        FileAction::Rename(path) => Action::RenameFile(path),
                        FileAction::Delete(path) => Action::DeleteFile(path),
                        FileAction::Refresh => Action::RefreshFiles,
                        FileAction::ShowFolder(path) => Action::ShowFolder(path),
                        FileAction::Move { from, to_dir } => Action::MovePath(from, to_dir),
                    });
                }
            }
            Tab::Editor(path) => editor_tab(ui, state, path),
            Tab::Console => {
                if let Some((path, line)) = state.console.ui(ui) {
                    state.actions.push(Action::OpenFile(path, Some(line)));
                }
            }
            Tab::Cpu => {
                let baseline = baseline
                    .as_deref()
                    .filter(|_| snapshot.state == RunState::Paused);
                let (min_width, rtc_interrupt) = views::machine::machine_ui(
                    ui,
                    &snapshot,
                    baseline,
                    state.emulator.program.as_deref(),
                );
                state.tools_min_width = min_width;
                if rtc_interrupt {
                    state.actions.push(Action::RtcInterrupt);
                }
            }
            Tab::Memory => {
                let baseline = baseline
                    .as_deref()
                    .filter(|_| snapshot.state == RunState::Paused);
                let follow = state.follows_execution();
                state
                    .memory_view
                    .ui(ui, &snapshot, baseline, &state.emulator.runner, follow);
            }
            Tab::Disassembly => {
                let program = state.emulator.program.clone();
                let follow = state.follows_execution();
                if let Some(address) = state
                    .disasm_view
                    .ui(ui, &snapshot, program.as_ref(), follow)
                {
                    state.actions.push(Action::ToggleAddressBreakpoint(address));
                }
            }
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
    find_bar(ui, state, Some(index));

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

    let generated = state.is_generated(path);
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
            search: (state.find.mode.is_some() && !state.find.query.is_empty())
                .then_some((state.find.query.as_str(), state.find.options)),
            current_match: state
                .find
                .current
                .as_ref()
                .filter(|(p, _)| p == path)
                .map(|(_, m)| m.clone()),
            signatures: (crate::highlight::Syntax::of(path) == crate::highlight::Syntax::C)
                .then_some(&state.signatures),
            font_size: state.settings.editor_font_size,
            read_only: state.snapshot.state == RunState::Running || generated,
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
