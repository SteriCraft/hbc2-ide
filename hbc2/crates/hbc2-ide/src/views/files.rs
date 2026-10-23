//! The project's files, as a folder tree.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};

use crate::project::Project;

pub enum FileAction {
    Open(PathBuf),
    NewFile(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    Refresh,
    ShowFolder,
}

#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<PathBuf>,
}

fn tree(project: &Project) -> Folder {
    let mut root = Folder::default();
    for path in &project.files {
        let relative = project.relative(path);
        let mut folder = &mut root;
        if let Some(parent) = relative.parent() {
            for part in parent.iter() {
                folder = folder
                    .folders
                    .entry(part.to_string_lossy().into())
                    .or_default();
            }
        }
        folder.files.push(path.clone());
    }
    root
}

pub fn files_ui(
    ui: &mut egui::Ui,
    project: Option<&Project>,
    is_open: &dyn Fn(&Path) -> bool,
    is_dirty: &dyn Fn(&Path) -> bool,
) -> Option<FileAction> {
    let Some(project) = project else {
        ui.label(RichText::new("No project open.").weak());
        ui.label("File > New project… or File > Open project…");
        return None;
    };
    let mut action = None;
    ui.horizontal(|ui| {
        ui.strong(project.name());
        if ui.small_button("+").on_hover_text("New file").clicked() {
            action = Some(FileAction::NewFile(project.root.clone()));
        }
        if ui.small_button("⟳").on_hover_text("Refresh").clicked() {
            action = Some(FileAction::Refresh);
        }
    });
    ui.separator();
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let root = tree(project);
        folder_ui(
            ui,
            &root,
            &project.root,
            project,
            is_open,
            is_dirty,
            &mut action,
        );
        // Right-click on empty space.
        let rest = ui.allocate_response(
            ui.available_size().max(egui::vec2(0.0, 20.0)),
            egui::Sense::click(),
        );
        rest.context_menu(|ui| {
            if ui.button("New file…").clicked() {
                action = Some(FileAction::NewFile(project.root.clone()));
            }
            if ui.button("Show folder").clicked() {
                action = Some(FileAction::ShowFolder);
            }
        });
    });
    action
}

fn folder_ui(
    ui: &mut egui::Ui,
    folder: &Folder,
    dir: &Path,
    project: &Project,
    is_open: &dyn Fn(&Path) -> bool,
    is_dirty: &dyn Fn(&Path) -> bool,
    action: &mut Option<FileAction>,
) {
    for (name, sub) in &folder.folders {
        let sub_dir = dir.join(name);
        let header = egui::CollapsingHeader::new(format!("🗁 {name}"))
            .id_salt(&sub_dir)
            .default_open(true)
            .show(ui, |ui| {
                folder_ui(ui, sub, &sub_dir, project, is_open, is_dirty, action)
            });
        header.header_response.context_menu(|ui| {
            if ui.button("New file…").clicked() {
                *action = Some(FileAction::NewFile(sub_dir.clone()));
            }
        });
    }
    for path in &folder.files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let entry = path == &project.entry();
        let mut text = RichText::new(format!("{}{name}", if is_dirty(path) { "• " } else { "" }));
        if entry {
            text = text.strong();
        }
        let response = ui.selectable_label(is_open(path), text);
        let response = if entry {
            response.on_hover_text("Entry file: the assembler starts here")
        } else {
            response
        };
        if response.clicked() {
            *action = Some(FileAction::Open(path.clone()));
        }
        response.context_menu(|ui| {
            if ui.button("Open").clicked() {
                *action = Some(FileAction::Open(path.clone()));
            }
            if ui.button("Rename…").clicked() {
                *action = Some(FileAction::Rename(path.clone()));
            }
            if ui
                .add_enabled(!entry, egui::Button::new("Delete…"))
                .clicked()
            {
                *action = Some(FileAction::Delete(path.clone()));
            }
            ui.separator();
            if ui.button("New file…").clicked() {
                *action = Some(FileAction::NewFile(
                    path.parent().unwrap_or(dir).to_path_buf(),
                ));
            }
        });
    }
}
