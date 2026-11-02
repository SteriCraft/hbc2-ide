//! Menu bar, toolbar, status bar, start page and dialogs.

use eframe::egui::{self, Align, Button, Color32, Layout, RichText};
use hbc2_asm::Target;
use hbc2_core::StopReason;
use hbc2_runner::{RunState, Speed};

use crate::app::{Action, Dialog, SettingsPage, State, can_undo};
use crate::find::FindMode;
use crate::project::{
    Language, folder_name, new_file_content, project_folder_name, source_file_name,
};
use crate::settings::{
    MonitorSettings, Placement, SPEEDS, Settings, TargetSetting, frequency_label, speed_label,
};
use crate::shortcuts::{self, Category, Command};

fn shortcut_button(text: &str, shortcut: String) -> Button<'static> {
    Button::new(text.to_string()).shortcut_text(shortcut)
}

/// Why a C project has no EEPROM target.
const C_STRIPS: &str = "C programs always leave out unused code";

fn build_label(c_project: bool) -> &'static str {
    if c_project { "Build" } else { "Assemble" }
}

fn is_c_project(state: &State) -> bool {
    state
        .project
        .as_ref()
        .is_some_and(|p| p.language == Language::C)
}

pub fn menu_bar(ui: &mut egui::Ui, state: &mut State) {
    let c_project = is_c_project(state);
    let building = state.is_building();
    let eeprom_target = state.target() == Target::Eeprom;
    let actions = &mut state.actions;
    let has_project = state.project.is_some();
    let keys = state.settings.shortcuts.clone();
    let key = |command| keys.text(command);
    egui::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New project…").clicked() {
                    actions.push(Action::NewProject);
                }
                if ui.add(shortcut_button("Open project…", key(Command::OpenProject))).clicked() {
                    actions.push(Action::OpenProjectDialog);
                }
                ui.add_enabled_ui(!state.settings.recent_projects.is_empty(), |ui| {
                    ui.menu_button("Recent projects", |ui| {
                        for root in &state.settings.recent_projects {
                            if ui.button(root.display().to_string()).clicked() {
                                actions.push(Action::OpenProject(root.clone()));
                            }
                        }
                    });
                });
                if ui
                    .add_enabled(has_project, Button::new("Close project"))
                    .clicked()
                {
                    actions.push(Action::CloseProject);
                }
                ui.separator();
                if ui
                    .add_enabled(has_project, Button::new("New file…"))
                    .clicked()
                    && let Some(project) = &state.project
                {
                    actions.push(Action::NewFile(project.root.clone()));
                }
                if ui.button("Open file…").clicked() {
                    actions.push(Action::OpenFileDialog);
                }
                if ui.add(shortcut_button("Save", key(Command::Save))).clicked() {
                    actions.push(Action::Save);
                }
                if ui
                    .add(shortcut_button("Save all", key(Command::SaveAll)))
                    .clicked()
                {
                    actions.push(Action::SaveAll);
                }
                ui.separator();
                if ui.button("Settings…").clicked() {
                    actions.push(Action::ShowDialog(Dialog::Settings {
                        page: SettingsPage::General,
                        draft: Box::new(state.settings.clone()),
                        capture: None,
                        notice: None,
                    }));
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                let editor = state.active_editor.is_some();
                let (can_undo, can_redo) = match &state.active_editor {
                    Some(path) => (
                        can_undo(&state.docs, ui.ctx(), path, false),
                        can_undo(&state.docs, ui.ctx(), path, true),
                    ),
                    None => (false, false),
                };
                if ui
                    .add_enabled(can_undo, shortcut_button("Undo", "Ctrl+Z".into()))
                    .clicked()
                {
                    actions.push(Action::Undo);
                }
                if ui
                    .add_enabled(can_redo, shortcut_button("Redo", "Ctrl+Y".into()))
                    .clicked()
                {
                    actions.push(Action::Redo);
                }
                ui.separator();
                if ui
                    .add_enabled(editor, shortcut_button("Find…", key(Command::Find)))
                    .clicked()
                {
                    actions.push(Action::OpenFind(FindMode::File));
                }
                if ui
                    .add_enabled(
                        has_project,
                        shortcut_button("Find in files…", key(Command::FindInFiles)),
                    )
                    .clicked()
                {
                    actions.push(Action::OpenFind(FindMode::Files));
                }
                ui.separator();
                if ui
                    .add_enabled(
                        editor,
                        shortcut_button("Collapse all functions", key(Command::FoldAll)),
                    )
                    .clicked()
                {
                    actions.push(Action::FoldAll(true));
                }
                if ui
                    .add_enabled(editor, shortcut_button("Expand all", key(Command::UnfoldAll)))
                    .clicked()
                {
                    actions.push(Action::FoldAll(false));
                }
            });

            ui.menu_button("Assembler", |ui| {
                if ui
                    .add_enabled(
                        has_project && !building,
                        shortcut_button(build_label(c_project), key(Command::Build)),
                    )
                    .clicked()
                {
                    actions.push(Action::Assemble);
                }
                ui.separator();
                ui.label(RichText::new("Memory target").weak());
                let target = state.project_settings.target;
                if ui
                    .radio(target == TargetSetting::Ram, "RAM (64 KiB image)")
                    .clicked()
                {
                    actions.push(Action::SetTarget(TargetSetting::Ram));
                }
                if ui
                    .radio(
                        target == TargetSetting::Eeprom,
                        "EEPROM (1 MiB image, boots into RAM)",
                    )
                    .clicked()
                {
                    actions.push(Action::SetTarget(TargetSetting::Eeprom));
                }
                ui.separator();
                let mut strip = state.project_settings.strip || c_project;
                if ui
                    .add_enabled(
                        !c_project,
                        egui::Checkbox::new(&mut strip, "Leave out unused code"),
                    )
                    .on_hover_text(
                        "Labels and variables nothing reaches from _start, a fixed address or .keep are not assembled",
                    )
                    .on_disabled_hover_text(C_STRIPS)
                    .changed()
                {
                    actions.push(Action::SetStrip(strip));
                }
                ui.separator();
                let loaded = state.emulator.image().is_some();
                if ui
                    .add_enabled(loaded, Button::new("Export image…"))
                    .clicked()
                {
                    actions.push(Action::ExportImage);
                }
            });

            ui.menu_button("Emulator", |ui| {
                let running = state.snapshot.state == RunState::Running;
                if ui
                    .add_enabled(!running, shortcut_button("Run", key(Command::Run)))
                    .clicked()
                {
                    actions.push(Action::Run);
                }
                if ui
                    .add_enabled(running, shortcut_button("Pause", key(Command::Pause)))
                    .clicked()
                {
                    actions.push(Action::Pause);
                }
                if ui
                    .add_enabled(!running, shortcut_button("Step over", key(Command::StepOver)))
                    .clicked()
                {
                    actions.push(Action::StepOver);
                }
                if ui
                    .add_enabled(!running, shortcut_button("Step into", key(Command::StepInto)))
                    .clicked()
                {
                    actions.push(Action::StepInto);
                }
                if ui
                    .add_enabled(!running, shortcut_button("Step out", key(Command::StepOut)))
                    .clicked()
                {
                    actions.push(Action::StepOut);
                }
                ui.checkbox(&mut state.settings.monitor_window, "Monitor window");
                if ui.add(shortcut_button("Reset", key(Command::Reset))).clicked() {
                    actions.push(Action::Reset);
                }
                ui.separator();
                ui.menu_button("Clock speed", |ui| {
                    let current: Speed = state.settings.speed.into();
                    for speed in SPEEDS {
                        if ui.radio(current == speed, speed_label(speed)).clicked() {
                            actions.push(Action::SetSpeed(speed));
                        }
                    }
                });
                ui.menu_button("Peripherals", |ui| {
                    let mut p = state.project_settings.peripherals;
                    let mut changed = ui.checkbox(&mut p.monitor, "Monitor").changed();
                    changed |= ui.checkbox(&mut p.rtc, "Real-time clock").changed();
                    changed |= ui.checkbox(&mut p.keyboard, "Keyboard").changed();
                    ui.add_enabled(
                        false,
                        egui::Checkbox::new(
                            &mut eeprom_target.clone(),
                            "EEPROM",
                        ),
                    )
                    .on_disabled_hover_text(
                        "Plugged in when the program is built for the EEPROM target",
                    );
                    ui.label(RichText::new("Changing them resets the machine.").weak());
                    if changed {
                        actions.push(Action::SetPeripherals(p));
                    }
                });
                ui.separator();
                if ui
                    .button("Load image…")
                    .on_hover_text("Run a RAM or EEPROM image file")
                    .clicked()
                {
                    actions.push(Action::LoadImage);
                }
                let eeprom = state.snapshot.eeprom.is_some();
                if ui
                    .add_enabled(eeprom, Button::new("Save EEPROM content…"))
                    .clicked()
                {
                    actions.push(Action::SaveEeprom);
                }
                ui.separator();
                if ui
                    .checkbox(
                        &mut state.settings.follow_pc,
                        "Follow the current instruction",
                    )
                    .changed()
                {
                    state.disasm_view.follow_pc = state.settings.follow_pc;
                    state.memory_view.follow_pc = state.settings.follow_pc;
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard map…").clicked() {
                    actions.push(Action::ShowDialog(Dialog::Keyboard));
                }
                if ui.button("About…").clicked() {
                    actions.push(Action::ShowDialog(Dialog::About));
                }
            });
        });
    });
}

pub fn toolbar(ui: &mut egui::Ui, state: &mut State) {
    let s = state.snapshot.clone();
    let running = s.state == RunState::Running;
    let has_project = state.project.is_some();
    let c_project = is_c_project(state);
    let building = state.is_building();
    let keys = state.settings.shortcuts.clone();
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            let actions = &mut state.actions;
            if ui
                .add_enabled(
                    has_project && !building,
                    Button::new(format!("🔨 {}", build_label(c_project))),
                )
                .on_hover_text(if c_project {
                    keys.with_key("Save, compile, assemble and load", Command::Build)
                } else {
                    keys.with_key("Assemble and load", Command::Build)
                })
                .clicked()
            {
                actions.push(Action::Assemble);
            }
            ui.separator();
            let run_label = if s.ticks > 0 { "▶ Continue" } else { "▶ Run" };
            if ui.add_enabled(!running, Button::new(run_label)).on_hover_text(keys.with_key("Run", Command::Run)).clicked() {
                actions.push(Action::Run);
            }
            if ui.add_enabled(running, Button::new("⏸ Pause")).on_hover_text(keys.with_key("Pause", Command::Pause)).clicked() {
                actions.push(Action::Pause);
            }
            if ui
                .add_enabled(!running, Button::new("⏭ Step"))
                .on_hover_text(format!("{}: the current line or instruction, with the routines it calls. Step into and out: Emulator menu", keys.with_key("Step over", Command::StepOver)))
                .clicked()
            {
                actions.push(Action::StepOver);
            }
            if ui
                .button("⟲ Reset")
                .on_hover_text(keys.with_key("Reset the machine: reload its image, paused", Command::Reset))
                .clicked()
            {
                actions.push(Action::Reset);
            }
            ui.separator();
            ui.toggle_value(&mut state.settings.monitor_window, "🖥 Monitor")
                .on_hover_text("Show or hide the monitor window");
            ui.separator();
            ui.label("Clock");
            let current: Speed = state.settings.speed.into();
            egui::ComboBox::from_id_salt("speed")
                .selected_text(speed_label(current))
                .width(90.0)
                .show_ui(ui, |ui| {
                    for speed in SPEEDS {
                        if ui.selectable_label(current == speed, speed_label(speed)).clicked() {
                            actions.push(Action::SetSpeed(speed));
                        }
                    }
                });
            ui.label("Target");
            let target = state.project_settings.target;
            egui::ComboBox::from_id_salt("target")
                .selected_text(match target {
                    TargetSetting::Ram => "RAM",
                    TargetSetting::Eeprom => "EEPROM",
                })
                .width(80.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(target == TargetSetting::Ram, "RAM").clicked() {
                        actions.push(Action::SetTarget(TargetSetting::Ram));
                    }
                    if ui.selectable_label(target == TargetSetting::Eeprom, "EEPROM").clicked() {
                        actions.push(Action::SetTarget(TargetSetting::Eeprom));
                    }
                });
            ui.separator();
            if building {
                ui.spinner();
                ui.label(RichText::new("Building…").strong())
                    .on_hover_text("The C build runs in the background. Run or Step pressed now start once it is loaded.");
                ui.separator();
            }
            let (text, color) = run_status(state);
            ui.label(RichText::new(text).color(color).strong());
            if state.is_stale() {
                ui.label(RichText::new("⚠ sources changed since the last build").color(ui.visuals().warn_fg_color))
                    .on_hover_text(format!("{} to load the new version. Run assembles by itself if the program has not started yet.", state.settings.shortcuts.with_key("Assemble", Command::Build)));
            }
        });
    });
}

fn run_status(state: &State) -> (String, Color32) {
    let s = &state.snapshot;
    let green = Color32::from_rgb(80, 180, 90);
    let amber = Color32::from_rgb(220, 160, 40);
    let red = Color32::from_rgb(220, 70, 60);
    let gray = Color32::GRAY;
    if state.emulator.image().is_none() {
        return ("Nothing loaded".into(), gray);
    }
    match s.state {
        RunState::Running if s.cpu.stopped_by_invalid_instruction() => (
            format!(
                "CPU stopped: invalid instruction at {:04X} (reset to restart)",
                s.cpu.last_pc
            ),
            red,
        ),
        RunState::Running if s.cpu.is_stopped() => (
            "CPU stopped: HLT with interrupts off (reset to restart)".into(),
            amber,
        ),
        RunState::Running if s.cpu.is_halted() => (
            "Running (CPU halted, waiting for an interrupt)".into(),
            green,
        ),
        RunState::Running => (
            format!("Running at {}", frequency_label(s.measured_hz)),
            green,
        ),
        RunState::Paused => match s.stop {
            Some(StopReason::Breakpoint(address)) => {
                (format!("Paused at breakpoint {address:04X}"), amber)
            }
            None if s.ticks == 0 => ("Ready (power-on)".into(), gray),
            None => (format!("Paused at {:04X}", s.cpu.pc), amber),
        },
    }
}

pub fn status_bar(ui: &mut egui::Ui, state: &State) {
    let s = &state.snapshot;
    egui::Panel::bottom("status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(position) = cursor_position(ui.ctx(), state) {
                ui.monospace(position);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.monospace(format!(
                    "{} cycles",
                    crate::views::cpu::group_digits(s.ticks)
                ));
                ui.separator();
                ui.monospace(format!("PC {:04X}", s.cpu.pc));
                ui.separator();
                let speed = match s.state {
                    RunState::Running => format!(
                        "{} / {}",
                        frequency_label(s.measured_hz),
                        speed_label(s.speed)
                    ),
                    RunState::Paused => format!("paused · target {}", speed_label(s.speed)),
                };
                ui.label(speed);
            });
        });
    });
}

/// "Ln 12, Col 5" for the cursor of the editor shown, with the number of
/// characters selected. Columns count a tab up to the next tab stop.
fn cursor_position(ctx: &egui::Context, state: &State) -> Option<String> {
    let path = state.active_editor.as_ref()?;
    let doc = state.docs.iter().find(|d| &d.path == path)?;
    let editor = egui::text_edit::TextEditState::load(ctx, crate::views::editor::editor_id(path))?;
    let range = editor.cursor.char_range()?;
    let (at, other) = (range.primary.index.0, range.secondary.index.0);
    let before: String = doc.text.chars().take(at).collect();
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let line = before.matches('\n').count() + 1;
    let column = crate::highlight::column(&before[line_start..]) + 1;
    let mut text = format!("Ln {line}, Col {column}");
    if at != other {
        text += &format!(" ({} selected)", at.abs_diff(other));
    }
    let cursors = crate::views::editor::cursor_count(ctx, path);
    if cursors > 1 {
        text += &format!(" · {cursors} cursors");
    }
    Some(text)
}

pub fn welcome(ui: &mut egui::Ui, state: &mut State) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.heading(RichText::new("HBC-2 IDE").size(28.0));
        ui.label("Write HBC-2 assembly or C, build it, and watch the computer run it.");
    });
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button("New project…").clicked() {
            state.actions.push(Action::NewProject);
        }
        if ui.button("Open project…").clicked() {
            state.actions.push(Action::OpenProjectDialog);
        }
    });
    if !state.settings.recent_projects.is_empty() {
        ui.add_space(12.0);
        ui.strong("Recent projects");
        for root in &state.settings.recent_projects {
            if ui.link(root.display().to_string()).clicked() {
                state.actions.push(Action::OpenProject(root.clone()));
            }
        }
    }
    ui.add_space(12.0);
    ui.strong("Keys");
    let keys = &state.settings.shortcuts;
    let key = |command| keys.text(command);
    let rows = [
        (
            key(Command::Build),
            "Assemble (or build a C project) and load",
        ),
        (key(Command::Run), "Run / continue"),
        (key(Command::Pause), "Pause"),
        (
            key(Command::StepOver),
            "Step over: the line or instruction, with the routines it calls",
        ),
        (
            key(Command::StepInto),
            "Step into: stop in the routines called",
        ),
        (key(Command::StepOut), "Step out: until the routine returns"),
        (key(Command::Reset), "Reset"),
        (key(Command::Save), "Save"),
        (key(Command::CloseTab), "Close the tab"),
        (keys.close_chord('R'), "Close the other tabs"),
        (keys.close_chord('K'), "Close all tabs"),
        (key(Command::Find), "Find in the file"),
        (key(Command::FindInFiles), "Find in the project's files"),
        ("Ctrl+click".into(), "Add a cursor (Ctrl+drag: a selection)"),
        (
            key(Command::SelectNext),
            "Select the word, then its next occurrence",
        ),
        (key(Command::SelectEvery), "Select every occurrence"),
        (
            key(Command::DuplicateLines),
            "Duplicate the lines of the cursors",
        ),
        ("Escape".into(), "Back to one cursor"),
        (key(Command::FoldAll), "Collapse all functions"),
        (key(Command::UnfoldAll), "Expand everything"),
    ];
    egui::Grid::new("keys").num_columns(2).show(ui, |ui| {
        for (key, what) in rows.iter().filter(|(key, _)| !key.is_empty()) {
            ui.monospace(key);
            ui.label(*what);
            ui.end_row();
        }
    });
    ui.label(RichText::new("Settings > Shortcuts changes them.").weak());
    ui.add_space(8.0);
    ui.label("Click in the margin of the editor to set a breakpoint. Click the monitor to type on the HBC-2 keyboard.");
}

/// Shows the open dialog, if any.
pub fn dialogs(ctx: &egui::Context, state: &mut State) {
    let Some(mut dialog) = state.dialog.take() else {
        return;
    };
    let mut keep = true;
    let language = match &state.project {
        Some(p) => p.language,
        None => Language::Assembly,
    };
    // Every dialog gets the same margins and spacing.
    let modal = egui::Modal::new(egui::Id::new("dialog")).show(ctx, |ui| {
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
        ui.set_max_width(460.0);
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.spacing_mut().button_padding = egui::vec2(10.0, 4.0);
        match &mut dialog {
            Dialog::Unsaved { paths, then } => {
                ui.heading("Unsaved changes");
                ui.label("These files have unsaved changes:");
                for path in paths.iter() {
                    ui.monospace(path.file_name().unwrap_or_default().to_string_lossy());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        keep = false;
                        if state.save(paths) {
                            state.actions.push((**then).clone());
                        }
                    }
                    if ui.button("Don't save").clicked() {
                        keep = false;
                        state.discard(paths);
                        state.actions.push((**then).clone());
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::NewProject {
                parent,
                own_folder,
                name,
                language,
                interrupt_handler,
                error,
            } => {
                ui.set_max_width(560.0);
                ui.heading("New project");
                ui.add_space(12.0);

                ui.label("Name");
                let edit = ui.add_enabled(
                    *own_folder,
                    egui::TextEdit::singleline(name).desired_width(380.0),
                );
                if *own_folder {
                    edit.request_focus();
                }
                ui.checkbox(own_folder, "Create a folder named after the project");
                ui.add_space(10.0);

                ui.label("Location");
                ui.horizontal(|ui| {
                    ui.monospace(parent.display().to_string());
                    if ui.button("Browse…").clicked() {
                        let mut picker = rfd::FileDialog::new()
                            .set_title("Folder where the project folder is created");
                        if parent.is_dir() {
                            picker = picker.set_directory(&*parent);
                        }
                        if let Some(dir) = picker.pick_folder() {
                            *parent = dir;
                            *error = None;
                        }
                    }
                });
                ui.add_space(10.0);

                ui.label("Language");
                ui.horizontal(|ui| {
                    ui.radio_value(language, Language::Assembly, "Assembly (main.has)");
                    ui.radio_value(language, Language::C, "C (main.c)");
                });
                ui.add_space(10.0);

                if *language == Language::Assembly {
                    ui.checkbox(interrupt_handler, "Include a default interrupt handler");
                } else {
                    ui.label(
                        RichText::new(
                            "Every .c and .has file of the folder is built. The startup \
                             code finds the devices before main runs.",
                        )
                        .weak(),
                    );
                }
                ui.add_space(10.0);

                let entry = match language {
                    Language::Assembly => crate::project::ENTRY_FILE,
                    Language::C => crate::project::C_ENTRY_FILE,
                };
                if !*own_folder {
                    let project = parent
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    ui.label(
                        RichText::new(format!(
                            "Creates the project {project} in {} (named after the folder).",
                            parent.display()
                        ))
                        .weak(),
                    );
                } else if let Ok(folder) = project_folder_name(name) {
                    ui.label(
                        RichText::new(format!("Creates {}", parent.join(folder).display()))
                            .weak(),
                    );
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let create = ui.button("Create").clicked() || enter;
                    if create && !*own_folder {
                        if !parent.is_dir() {
                            *error = Some("The location folder does not exist.".into());
                        } else if parent.join(entry).exists() {
                            *error = Some(format!("This folder already holds a {entry}."));
                        } else {
                            keep = false;
                            state.actions.push(Action::CreateProject(
                                parent.clone(),
                                *language,
                                *interrupt_handler,
                            ));
                        }
                    } else if create {
                        match project_folder_name(name) {
                            Err(e) => *error = Some(e.into()),
                            Ok(_) if !parent.is_dir() => {
                                *error = Some("The location folder does not exist.".into())
                            }
                            Ok(folder) => {
                                let root = parent.join(folder);
                                if root.exists() {
                                    *error =
                                        Some("A folder with this name already exists.".into());
                                } else {
                                    keep = false;
                                    state
                                        .actions
                                        .push(Action::CreateProject(
                                            root,
                                            *language,
                                            *interrupt_handler,
                                        ));
                                }
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::NewFile {
                dir,
                name,
                error,
                select_stem,
            } => {
                ui.heading("New file");
                ui.label(format!("In {}", dir.display()));
                let edit = ui.text_edit_singleline(name);
                edit.request_focus();
                if std::mem::take(select_stem) {
                    select_name_stem(ui.ctx(), edit.id, name);
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() || enter {
                        match source_file_name(name, language) {
                            Err(e) => *error = Some(e.into()),
                            Ok(file) => {
                                let path = dir.join(&file);
                                if path.exists() {
                                    *error = Some("A file with this name already exists.".into());
                                } else {
                                    match std::fs::write(&path, new_file_content(&file)) {
                                        Ok(()) => {
                                            keep = false;
                                            state.actions.push(Action::RefreshFiles);
                                            state.actions.push(Action::OpenFile(path, None));
                                        }
                                        Err(e) => *error = Some(e.to_string()),
                                    }
                                }
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
                let file =
                    source_file_name(name, language).unwrap_or_else(|_| name.trim().to_string());
                let include = match &state.project {
                    Some(project) => project.include_path(&dir.join(&file)),
                    None => file,
                };
                let hint = if language == Language::C {
                    "Every .c and .has file of the project is built.".to_string()
                } else {
                    format!("Include it from main.has with .include \"{include}\".")
                };
                ui.label(RichText::new(hint).weak());
            }
            Dialog::NewFolder { dir, name, error } => {
                ui.heading("New folder");
                ui.label(format!("In {}", dir.display()));
                let edit = ui.text_edit_singleline(name);
                edit.request_focus();
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() || enter {
                        match folder_name(name) {
                            Err(e) => *error = Some(e.into()),
                            Ok(folder) => {
                                let path = dir.join(folder);
                                if path.exists() {
                                    *error = Some("A file or folder with this name already exists.".into());
                                } else {
                                    match std::fs::create_dir(&path) {
                                        Ok(()) => {
                                            keep = false;
                                            state.actions.push(Action::RefreshFiles);
                                        }
                                        Err(e) => *error = Some(e.to_string()),
                                    }
                                }
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::Rename {
                path,
                name,
                error,
                select_stem,
            } => {
                ui.heading("Rename");
                let edit = ui.text_edit_singleline(name);
                edit.request_focus();
                if std::mem::take(select_stem) {
                    select_name_stem(ui.ctx(), edit.id, name);
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() || enter {
                        match source_file_name(name, language) {
                            Err(e) => *error = Some(e.into()),
                            Ok(file) => {
                                let new_path = path.with_file_name(file);
                                if new_path.exists() {
                                    *error = Some("A file with this name already exists.".into());
                                } else if let Err(e) = std::fs::rename(&*path, &new_path) {
                                    *error = Some(e.to_string());
                                } else {
                                    keep = false;
                                    state.renamed(path, new_path);
                                }
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
                ui.label(RichText::new(".include lines that use the old name must be updated by hand.").weak());
            }
            Dialog::Delete { path } => {
                ui.heading("Delete file");
                ui.label(format!("Delete {} from the disk?", path.display()));
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        keep = false;
                        match std::fs::remove_file(&*path) {
                            Ok(()) => {
                                state.console.info(format!("Deleted {}.", path.display()));
                                state.docs.retain(|d| &d.path != path);
                                state.actions.push(Action::CloseTab(path.clone()));
                                state.actions.push(Action::RefreshFiles);
                            }
                            Err(e) => state.console.error(format!("Cannot delete {}: {e}", path.display())),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::Message { title, text } => {
                ui.heading(title.as_str());
                ui.label(text.as_str());
                if ui.button("OK").clicked() {
                    keep = false;
                }
            }
            Dialog::About => {
                ui.heading("HBC-2 IDE");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("Integrated development environment and emulator for the HBC-2 (HomeBrew Computer 2).");
                ui.label("Created by Gianni Leclercq.");
                ui.hyperlink_to("GitHub repository", "https://github.com/SteriCraft/hbc2-ide");
                ui.hyperlink_to("Project website", "https://hbc2fr.wordpress.com/");
                ui.hyperlink_to("License: GPL 3.0", "https://www.gnu.org/licenses/gpl-3.0.html");
                if ui.button("Close").clicked() {
                    keep = false;
                }
            }
            Dialog::Keyboard => {
                keyboard_help(ui);
                if ui.button("Close").clicked() {
                    keep = false;
                }
            }
            Dialog::Settings {
                page,
                draft,
                capture,
                notice,
            } => {
                ui.set_max_width(620.0);
                ui.heading("Settings");
                ui.add_space(12.0);
                // Categories on the left, the chosen one on the right, with
                // a line between them as high as the content (a separator
                // widget would take all the height it can).
                let row = ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(110.0);
                        for choice in SettingsPage::ALL {
                            ui.selectable_value(page, choice, choice.label());
                        }
                    });
                    let line = ui.cursor().left() + 8.0;
                    ui.add_space(17.0);
                    ui.vertical(|ui| {
                        // The same size for every page: the dialog does not jump.
                        ui.set_min_size(egui::vec2(400.0, 280.0));
                        match page {
                            SettingsPage::General => general_settings(ui, draft),
                            SettingsPage::Editor => editor_settings(ui, draft),
                            SettingsPage::Build => build_settings(ui, draft),
                            SettingsPage::Emulator => emulator_settings(ui, draft),
                            SettingsPage::Monitor => monitor_settings(ui, &mut draft.monitor),
                            SettingsPage::Shortcuts => {
                                shortcut_settings(ui, &mut draft.shortcuts, capture, notice)
                            }
                        }
                    });
                    line
                });
                ui.painter().vline(
                    row.inner,
                    row.response.rect.y_range(),
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                if let Some(command) = *capture {
                    capture_dialog(ui.ctx(), command, &mut draft.shortcuts, capture, notice);
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        keep = false;
                        state.settings.set_preferences(draft);
                        state.disasm_view.follow_pc = state.settings.follow_pc;
                        state.memory_view.follow_pc = state.settings.follow_pc;
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
        }
        });
    });
    if modal.should_close() && !matches!(dialog, Dialog::Unsaved { .. }) {
        keep = false;
    }
    if keep && state.dialog.is_none() {
        state.dialog = Some(dialog);
    }
}

/// A title above a group of settings.
fn settings_section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong());
}

fn general_settings(ui: &mut egui::Ui, settings: &mut Settings) {
    let general = &mut settings.general;
    settings_section(ui, "Startup");
    ui.checkbox(
        &mut general.reopen_last_project,
        "Reopen the last project, with its files",
    );
    settings_section(ui, "New projects");
    ui.checkbox(
        &mut general.project_folder,
        "Create a directory named after the project by default",
    )
    .on_hover_text(
        "Unticked, the project goes right in the chosen folder, and takes the folder's name.",
    );
    ui.label("Folder for new projects");
    ui.horizontal(|ui| {
        let folder = settings.projects_folder();
        ui.monospace(folder.display().to_string());
        if ui.button("Browse…").clicked()
            && let Some(dir) = rfd::FileDialog::new()
                .set_title("Folder for new projects")
                .set_directory(&folder)
                .pick_folder()
        {
            settings.general.projects_folder = Some(dir);
        }
        if settings.general.projects_folder.is_some()
            && ui
                .button("Default")
                .on_hover_text("Documents/hbc2_projects")
                .clicked()
        {
            settings.general.projects_folder = None;
        }
    });
    settings_section(ui, "Recent projects");
    ui.horizontal(|ui| {
        ui.label("File > Recent projects lists");
        ui.add(
            egui::DragValue::new(&mut settings.general.recent_count)
                .range(1..=30)
                .speed(0.1),
        );
        ui.label("projects");
    });
}

fn editor_settings(ui: &mut egui::Ui, settings: &mut Settings) {
    settings_section(ui, "Text");
    ui.horizontal(|ui| {
        ui.label("Font size");
        ui.add(
            egui::DragValue::new(&mut settings.editor_font_size)
                .range(8.0..=32.0)
                .suffix(" px"),
        );
    });
    let editor = &mut settings.editor;
    settings_section(ui, "Indentation");
    ui.horizontal(|ui| {
        ui.label("Tab key inserts");
        ui.radio_value(&mut editor.indent_with_spaces, false, "Tabs");
        ui.radio_value(&mut editor.indent_with_spaces, true, "Spaces");
    });
    ui.horizontal(|ui| {
        ui.label("A tab is as wide as");
        ui.add(
            egui::DragValue::new(&mut editor.tab_width)
                .range(1..=16)
                .speed(0.05),
        );
        ui.label(if editor.tab_width == 1 {
            "space"
        } else {
            "spaces"
        });
    })
    .response
    .on_hover_text("Also the number of spaces Tab inserts when it inserts spaces.");
    settings_section(ui, "Display");
    ui.checkbox(&mut editor.line_numbers, "Show line numbers");
    ui.checkbox(
        &mut editor.highlight_line,
        "Highlight the line of the cursor",
    );
}

fn build_settings(ui: &mut egui::Ui, settings: &mut Settings) {
    let build = &mut settings.build;
    ui.checkbox(
        &mut build.save_before_build,
        "Save the files before assembling",
    )
    .on_hover_text(
        "Assembly projects are otherwise assembled from the editors, unsaved changes \
         included. C projects are always saved before they are built.",
    );
    ui.checkbox(
        &mut build.run_after_build,
        "Run the program after a successful build",
    );
}

fn emulator_settings(ui: &mut egui::Ui, settings: &mut Settings) {
    settings_section(ui, "Startup");
    ui.horizontal(|ui| {
        ui.label("Clock");
        let emulator = &mut settings.emulator;
        let shown = match emulator.startup_speed {
            Some(speed) => speed_label(speed.into()),
            None => "As last used".into(),
        };
        egui::ComboBox::from_id_salt("startup_speed")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut emulator.startup_speed, None, "As last used");
                for speed in SPEEDS {
                    ui.selectable_value(
                        &mut emulator.startup_speed,
                        Some(speed.into()),
                        speed_label(speed),
                    );
                }
            });
    });
    settings_section(ui, "Debugging");
    ui.checkbox(&mut settings.follow_pc, "Follow the current instruction")
        .on_hover_text(
            "When the emulator pauses, show the line, disassembly and memory of the next \
             instruction.",
        );
    settings_section(ui, "Peripherals of new projects");
    let peripherals = &mut settings.emulator.new_project_peripherals;
    ui.checkbox(&mut peripherals.monitor, "Monitor");
    ui.checkbox(&mut peripherals.rtc, "Real-time clock");
    ui.checkbox(&mut peripherals.keyboard, "Keyboard");
}

fn shortcut_settings(
    ui: &mut egui::Ui,
    keys: &mut shortcuts::Shortcuts,
    capture: &mut Option<Command>,
    notice: &mut Option<String>,
) {
    ui.label(RichText::new("Click a shortcut to change it.").weak());
    let fixed = |ui: &mut egui::Ui, what: &str, keys: &str, why: &str| {
        ui.label(RichText::new(what).weak());
        ui.label(RichText::new(keys).monospace().weak())
            .on_hover_text(why);
        ui.end_row();
    };
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for category in Category::ALL {
                settings_section(ui, category.label());
                egui::Grid::new(("shortcuts", category.label()))
                    .num_columns(2)
                    .min_col_width(190.0)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for command in Command::ALL
                            .into_iter()
                            .filter(|c| c.category() == category)
                        {
                            ui.label(command.label());
                            let text = keys.text(command);
                            let shown = if text.is_empty() {
                                RichText::new("None").weak().italics()
                            } else {
                                RichText::new(text).monospace()
                            };
                            if ui
                                .add(Button::new(shown).min_size(egui::vec2(140.0, 0.0)))
                                .on_hover_text("Click, then press the new shortcut")
                                .clicked()
                            {
                                *capture = Some(command);
                                *notice = None;
                            }
                            ui.end_row();
                        }
                        if category == Category::File && keys.get(Command::CloseTab).is_some() {
                            let why = "Hold Close tab and press the letter";
                            fixed(ui, "Close the other tabs", &keys.close_chord('R'), why);
                            fixed(ui, "Close all tabs", &keys.close_chord('K'), why);
                        }
                        for (_, what, text) in shortcuts::FIXED.iter().filter(|f| f.0 == category) {
                            fixed(ui, what, text, "Built into the text box: cannot be changed");
                        }
                    });
            }
        });
    ui.add_space(6.0);
    if let Some(text) = notice {
        ui.label(RichText::new(text.as_str()).color(ui.visuals().warn_fg_color));
    }
    if ui
        .add_enabled(!keys.is_default(), Button::new("Restore defaults"))
        .clicked()
    {
        keys.restore_defaults();
        *notice = None;
    }
}

/// The small dialog that waits for the new shortcut of `command`.
fn capture_dialog(
    ctx: &egui::Context,
    command: Command,
    keys: &mut shortcuts::Shortcuts,
    capture: &mut Option<Command>,
    notice: &mut Option<String>,
) {
    use egui::{Event, Key, KeyboardShortcut};
    let taken = |other: Option<Command>, shortcut: Option<KeyboardShortcut>| {
        let (other, shortcut) = (other?, shortcut?);
        Some(format!(
            "{} was taken from \u{201c}{}\u{201d}, which has no shortcut now.",
            shortcuts::format(shortcut),
            other.label()
        ))
    };
    let mut done = false;
    let modal = egui::Modal::new(egui::Id::new("shortcut-capture")).show(ctx, |ui| {
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
            ui.set_width(340.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 4.0);
            ui.heading(command.label());
            ui.label("Press the new shortcut…");
            let now = keys.text(command);
            ui.label(
                RichText::new(format!(
                    "Now: {}",
                    if now.is_empty() { "none" } else { now.as_str() }
                ))
                .weak(),
            );
            ui.label(RichText::new("With Ctrl or Alt, or a function key. Escape cancels.").weak());
            // The keys go to this dialog only.
            let pressed = ui.input_mut(|i| {
                let mut found = None;
                i.events.retain(|event| match event {
                    Event::Key {
                        key,
                        pressed,
                        modifiers,
                        ..
                    } => {
                        if *pressed && !shortcuts::is_modifier(*key) && found.is_none() {
                            found = Some(KeyboardShortcut::new(*modifiers, *key));
                        }
                        false
                    }
                    Event::Text(_) => false,
                    _ => true,
                });
                found
            });
            match pressed {
                Some(s) if s.logical_key == Key::Escape && s.modifiers.is_none() => {
                    *notice = None;
                    done = true;
                }
                Some(s) => match shortcuts::refusal(s) {
                    Some(why) => *notice = Some(why),
                    None => {
                        *notice = taken(keys.assign(command, Some(s)), Some(s));
                        done = true;
                    }
                },
                None => {}
            }
            if let Some(text) = notice {
                ui.label(RichText::new(text.as_str()).color(ui.visuals().warn_fg_color));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("No shortcut").clicked() {
                    keys.assign(command, None);
                    *notice = None;
                    done = true;
                }
                let default = command.default_shortcut();
                let default_text = default.map_or_else(|| "None".into(), shortcuts::format);
                if ui
                    .add_enabled(keys.get(command) != default, Button::new("Default"))
                    .on_hover_text(default_text)
                    .clicked()
                {
                    *notice = taken(keys.assign(command, default), default);
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    *notice = None;
                    done = true;
                }
            });
        });
    });
    if done || modal.should_close() {
        *capture = None;
    }
}

fn monitor_settings(ui: &mut egui::Ui, monitor: &mut MonitorSettings) {
    ui.checkbox(
        &mut monitor.open_on_startup,
        "Open the monitor window on startup",
    );
    ui.checkbox(&mut monitor.sharp_scaling, "Sharp scaling on startup")
        .on_hover_text("Scale by whole numbers only, so every HBC-2 pixel has the same size.");
    ui.add_enabled_ui(monitor.sharp_scaling, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label("Pixel size");
            ui.add(
                egui::DragValue::new(&mut monitor.pixel_size)
                    .range(1..=8)
                    .speed(0.05)
                    .suffix(" px"),
            );
        })
        .response
        .on_hover_text("Size of an HBC-2 pixel when the monitor window opens.");
    });
    ui.add_space(10.0);
    ui.checkbox(
        &mut monitor.remember_position,
        "Open the monitor window where it was last closed",
    );
    ui.add_enabled_ui(!monitor.remember_position, |ui| {
        ui.label("Where the monitor window opens on the screen");
        ui.horizontal(|ui| {
            placement_picker(ui, &mut monitor.placement);
            ui.label(RichText::new(monitor.placement.label()).weak());
        });
    });
}

/// The screen as 3×3 squares: a click picks where the window goes.
fn placement_picker(ui: &mut egui::Ui, placement: &mut Placement) {
    let cell = egui::vec2(36.0, 27.0);
    let gap = 3.0;
    let size = cell * 3.0 + egui::vec2(gap, gap) * 4.0;
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let visuals = ui.visuals().clone();
    ui.painter().rect(
        rect,
        4.0,
        visuals.extreme_bg_color,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    for (index, &choice) in Placement::ALL.iter().enumerate() {
        let (column, row) = ((index % 3) as f32, (index / 3) as f32);
        let min = rect.min + egui::vec2(gap + column * (cell.x + gap), gap + row * (cell.y + gap));
        let square = egui::Rect::from_min_size(min, cell);
        let id = ui.id().with(("placement", index));
        let response = ui
            .interact(square, id, egui::Sense::click())
            .on_hover_text(choice.label());
        if response.clicked() {
            *placement = choice;
        }
        let fill = if *placement == choice {
            visuals.selection.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.weak_bg_fill
        } else {
            visuals.widgets.inactive.weak_bg_fill
        };
        ui.painter().rect_filled(square, 2.0, fill);
    }
}

fn keyboard_help(ui: &mut egui::Ui) {
    ui.heading("HBC-2 keyboard");
    ui.label(
        "Click the monitor, then type: each key sends its scan code to the HBC-2 (an interrupt on the \
         keyboard's first port when pressed, on its second port when released).",
    );
    ui.label(
        "The HBC-2 keyboard is AZERTY: a key's scan code is the ASCII code of what it shows on an AZERTY \
         keyboard (unshifted, capital letters), whatever your own layout is. Shift, Ctrl and Alt are keys \
         of their own; the program decides what they do.",
    );
    ui.label(
        "Caps Lock holds Shift down on the HBC-2 keyboard. The é, è, à and ù keys send E, E, A and U, so \
         French text still reads well without accents.",
    );
    ui.add_space(6.0);
    egui::Grid::new("special_keys")
        .striped(true)
        .num_columns(4)
        .show(ui, |ui| {
            let keys = [
                ("Insert", 0x01),
                ("Home", 0x02),
                ("End", 0x03),
                ("Page Up", 0x04),
                ("Page Down", 0x05),
                ("Left", 0x07),
                ("Backspace", 0x08),
                ("Tab", 0x09),
                ("Up", 0x0A),
                ("Right", 0x0B),
                ("Down", 0x0C),
                ("Enter", 0x0D),
                ("Shift", 0x0E),
                ("Ctrl", 0x10),
                ("Alt", 0x11),
                ("Keypad +", 0x13),
                ("Keypad -", 0x14),
                ("Keypad /", 0x16),
                ("Escape", 0x1B),
                ("Delete", 0x7F),
            ];
            for pair in keys.chunks(2) {
                for (name, code) in pair {
                    ui.label(*name);
                    ui.monospace(format!("{code:02X}"));
                }
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Not available: Pause, keypad Enter, * and . (they look like other keys to the IDE).",
        )
        .weak(),
    );
}

impl State {
    /// A file was renamed on disk: update the editor, breakpoints and file list.
    fn renamed(&mut self, old: &std::path::Path, new: std::path::PathBuf) {
        for doc in self.docs.iter_mut().filter(|d| d.path == old) {
            doc.set_path(new.clone());
        }
        if let Some(lines) = self.project_settings.breakpoints.remove(old) {
            self.project_settings.breakpoints.insert(new.clone(), lines);
        }
        self.actions.push(Action::CloseTab(old.to_path_buf()));
        self.actions.push(Action::OpenFile(new, None));
        self.actions.push(Action::RefreshFiles);
        self.edits += 1;
    }
}

/// Selects `name` in the text field `id` up to its last '.', so typing
/// replaces the name and keeps the extension.
fn select_name_stem(ctx: &egui::Context, id: egui::Id, name: &str) {
    use egui::text::{CCursor, CCursorRange};
    let stem = name.rfind('.').map_or(name, |dot| &name[..dot]);
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(stem.chars().count()),
    )));
    egui::TextEdit::store_state(ctx, id, state);
}
