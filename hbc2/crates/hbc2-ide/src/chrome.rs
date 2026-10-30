//! Menu bar, toolbar, status bar, start page and dialogs.

use eframe::egui::{self, Align, Button, Color32, Layout, RichText};
use hbc2_core::StopReason;
use hbc2_runner::{RunState, Speed};

use crate::app::{Action, Dialog, State, can_undo};
use crate::find::FindMode;
use crate::project::{NEW_FILE_TEMPLATE, folder_name, project_folder_name, source_file_name};
use crate::settings::{SPEEDS, TargetSetting, frequency_label, speed_label};

fn shortcut_button(text: &str, shortcut: &str) -> Button<'static> {
    Button::new(text.to_string()).shortcut_text(shortcut.to_string())
}

pub fn menu_bar(ui: &mut egui::Ui, state: &mut State) {
    let actions = &mut state.actions;
    let has_project = state.project.is_some();
    egui::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New project…").clicked() {
                    actions.push(Action::NewProject);
                }
                if ui.add(shortcut_button("Open project…", "Ctrl+O")).clicked() {
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
                if ui.add(shortcut_button("Save", "Ctrl+S")).clicked() {
                    actions.push(Action::Save);
                }
                if ui
                    .add(shortcut_button("Save all", "Ctrl+Shift+S"))
                    .clicked()
                {
                    actions.push(Action::SaveAll);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Editor font");
                    ui.add(
                        egui::DragValue::new(&mut state.settings.editor_font_size)
                            .range(8.0..=32.0)
                            .suffix(" px"),
                    );
                });
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
                    .add_enabled(can_undo, shortcut_button("Undo", "Ctrl+Z"))
                    .clicked()
                {
                    actions.push(Action::Undo);
                }
                if ui
                    .add_enabled(can_redo, shortcut_button("Redo", "Ctrl+Y"))
                    .clicked()
                {
                    actions.push(Action::Redo);
                }
                ui.separator();
                if ui
                    .add_enabled(editor, shortcut_button("Find…", "Ctrl+F"))
                    .clicked()
                {
                    actions.push(Action::OpenFind(FindMode::File));
                }
                if ui
                    .add_enabled(
                        has_project,
                        shortcut_button("Find in files…", "Ctrl+Shift+F"),
                    )
                    .clicked()
                {
                    actions.push(Action::OpenFind(FindMode::Files));
                }
            });

            ui.menu_button("Assembler", |ui| {
                if ui
                    .add_enabled(has_project, shortcut_button("Assemble", "F7"))
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
                    .add_enabled(!running, shortcut_button("Run", "F5"))
                    .clicked()
                {
                    actions.push(Action::Run);
                }
                if ui
                    .add_enabled(running, shortcut_button("Pause", "F6"))
                    .clicked()
                {
                    actions.push(Action::Pause);
                }
                if ui
                    .add_enabled(!running, shortcut_button("Step", "F10"))
                    .clicked()
                {
                    actions.push(Action::Step);
                }
                ui.checkbox(&mut state.settings.monitor_window, "Monitor window");
                if ui.add(shortcut_button("Power cycle", "Ctrl+F5")).clicked() {
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
                            &mut (state.project_settings.target == TargetSetting::Eeprom),
                            "EEPROM",
                        ),
                    )
                    .on_disabled_hover_text(
                        "Plugged in when the program is assembled for the EEPROM target",
                    );
                    ui.label(RichText::new("Changing them restarts the machine.").weak());
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
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            let actions = &mut state.actions;
            if ui
                .add_enabled(has_project, Button::new("🔨 Assemble"))
                .on_hover_text("Assemble and load (F7)")
                .clicked()
            {
                actions.push(Action::Assemble);
            }
            ui.separator();
            let run_label = if s.ticks > 0 { "▶ Continue" } else { "▶ Run" };
            if ui.add_enabled(!running, Button::new(run_label)).on_hover_text("F5").clicked() {
                actions.push(Action::Run);
            }
            if ui.add_enabled(running, Button::new("⏸ Pause")).on_hover_text("F6").clicked() {
                actions.push(Action::Pause);
            }
            if ui
                .add_enabled(!running, Button::new("⏭ Step"))
                .on_hover_text("Run one instruction (F10)")
                .clicked()
            {
                actions.push(Action::Step);
            }
            if ui
                .button("⟲ Power cycle")
                .on_hover_text("Restart the machine from its image (Ctrl+F5)")
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
            let (text, color) = run_status(state);
            ui.label(RichText::new(text).color(color).strong());
            if state.is_stale() {
                ui.label(RichText::new("⚠ sources changed since the last assembly").color(ui.visuals().warn_fg_color))
                    .on_hover_text("Assemble (F7) to load the new version. Run assembles by itself if the program has not started yet.");
            }
        });
    });
}

fn run_status(state: &State) -> (String, Color32) {
    let s = &state.snapshot;
    let green = Color32::from_rgb(80, 180, 90);
    let amber = Color32::from_rgb(220, 160, 40);
    let gray = Color32::GRAY;
    if state.emulator.image().is_none() {
        return ("Nothing loaded".into(), gray);
    }
    match s.state {
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
            match &state.project {
                Some(p) => ui.label(p.root.display().to_string()),
                None => ui.weak("No project"),
            };
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

pub fn welcome(ui: &mut egui::Ui, state: &mut State) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.heading(RichText::new("HBC-2 IDE").size(28.0));
        ui.label("Write HBC-2 assembly, assemble it, and watch the computer run it.");
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
    egui::Grid::new("keys").num_columns(2).show(ui, |ui| {
        for (key, what) in [
            ("F7", "Assemble and load"),
            ("F5", "Run / continue"),
            ("F6", "Pause"),
            ("F10", "Step one instruction"),
            ("Ctrl+F5", "Power cycle"),
            ("Ctrl+S", "Save"),
            ("Ctrl+W", "Close the tab"),
            ("Ctrl+W+R", "Close the other tabs"),
            ("Ctrl+W+K", "Close all tabs"),
            ("Ctrl+F", "Find in the file"),
            ("Ctrl+Shift+F", "Find in the project's files"),
        ] {
            ui.monospace(key);
            ui.label(what);
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    ui.label("Click in the margin of the editor to set a breakpoint. Click the monitor to type on the HBC-2 keyboard.");
}

/// Shows the open dialog, if any.
pub fn dialogs(ctx: &egui::Context, state: &mut State) {
    let Some(mut dialog) = state.dialog.take() else {
        return;
    };
    let mut keep = true;
    let modal = egui::Modal::new(egui::Id::new("dialog")).show(ctx, |ui| {
        ui.set_max_width(460.0);
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
                name,
                interrupt_handler,
                error,
            } => {
                egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
                    ui.set_max_width(560.0);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 4.0);
                    ui.heading("New project");
                    ui.add_space(12.0);

                    ui.label("Name");
                    let edit = ui.add(egui::TextEdit::singleline(name).desired_width(380.0));
                    edit.request_focus();
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

                    ui.checkbox(interrupt_handler, "Include a default interrupt handler");
                    ui.add_space(10.0);

                    if let Ok(folder) = project_folder_name(name) {
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
                        if ui.button("Create").clicked() || enter {
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
                                            .push(Action::CreateProject(root, *interrupt_handler));
                                    }
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                });
            }
            Dialog::NewFile { dir, name, error } => {
                ui.heading("New file");
                ui.label(format!("In {}", dir.display()));
                let edit = ui.text_edit_singleline(name);
                edit.request_focus();
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() || enter {
                        match source_file_name(name) {
                            Err(e) => *error = Some(e.into()),
                            Ok(file) => {
                                let path = dir.join(file);
                                if path.exists() {
                                    *error = Some("A file with this name already exists.".into());
                                } else {
                                    match std::fs::write(&path, NEW_FILE_TEMPLATE) {
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
                let file = source_file_name(name).unwrap_or_else(|_| name.trim().to_string());
                let include = match &state.project {
                    Some(project) => project.include_path(&dir.join(&file)),
                    None => file,
                };
                ui.label(
                    RichText::new(format!("Include it from main.has with .include \"{include}\"."))
                        .weak(),
                );
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
            Dialog::Rename { path, name, error } => {
                ui.heading("Rename");
                let edit = ui.text_edit_singleline(name);
                edit.request_focus();
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() || enter {
                        match source_file_name(name) {
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
        }
    });
    if modal.should_close() && !matches!(dialog, Dialog::Unsaved { .. }) {
        keep = false;
    }
    if keep && state.dialog.is_none() {
        state.dialog = Some(dialog);
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
    ui.label(RichText::new("Not available: Pause, Caps Lock, keypad Enter, * and . (they look like other keys to the IDE).").weak());
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
