//! The project's files, as a folder tree.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, DragAndDrop, RichText, Sense, StrokeKind};

use crate::project::Project;

pub enum FileAction {
    Open(PathBuf),
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    Refresh,
    /// Opens the file manager on a folder, or on a file's folder.
    ShowFolder(PathBuf),
    /// Moves a file or folder into another folder.
    Move {
        from: PathBuf,
        to_dir: PathBuf,
    },
}

/// A file or folder being dragged in the tree.
struct Dragged(PathBuf);

/// Whether `from` can move into `dir`: not where it already is, and a folder
/// not into itself.
fn can_move(from: &Path, dir: &Path) -> bool {
    from.parent() != Some(dir) && !dir.starts_with(from)
}

/// Makes `response` a drag source for `path`.
fn drag_source(response: egui::Response, path: &Path) -> egui::Response {
    let response = response.interact(Sense::drag());
    response.dnd_set_drag_payload(Dragged(path.to_path_buf()));
    response
}

/// Makes `response` a place to drop files and folders into `dir`.
fn drop_target(
    ui: &egui::Ui,
    response: &egui::Response,
    dir: &Path,
    action: &mut Option<FileAction>,
) {
    if let Some(dragged) = response.dnd_hover_payload::<Dragged>()
        && can_move(&dragged.0, dir)
    {
        let stroke = ui.visuals().selection.stroke;
        ui.painter()
            .rect_stroke(response.rect, 2.0, stroke, StrokeKind::Inside);
    }
    if let Some(dragged) = response.dnd_release_payload::<Dragged>()
        && can_move(&dragged.0, dir)
    {
        *action = Some(FileAction::Move {
            from: dragged.0.clone(),
            to_dir: dir.to_path_buf(),
        });
    }
}

/// The name of what is being dragged, next to the pointer.
fn drag_preview(ui: &egui::Ui) {
    let Some(dragged) = DragAndDrop::payload::<Dragged>(ui.ctx()) else {
        return;
    };
    let Some(pos) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    let name = dragged.0.file_name().unwrap_or_default().to_string_lossy();
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("dragged_file"),
    ));
    let text = painter.layout_no_wrap(
        name.into(),
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().strong_text_color(),
    );
    let rect = egui::Rect::from_min_size(pos + egui::vec2(14.0, 4.0), text.size()).expand(4.0);
    painter.rect_filled(rect, 3.0, ui.visuals().window_fill);
    painter.rect_stroke(rect, 3.0, ui.visuals().window_stroke, StrokeKind::Inside);
    painter.galley(
        rect.min + egui::vec2(4.0, 4.0),
        text,
        egui::Color32::PLACEHOLDER,
    );
}

#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<PathBuf>,
}

impl Folder {
    /// The subfolder at `relative`, created in the tree if needed.
    fn at(&mut self, relative: &Path) -> &mut Folder {
        let mut folder = self;
        for part in relative.iter() {
            folder = folder
                .folders
                .entry(part.to_string_lossy().into())
                .or_default();
        }
        folder
    }
}

fn tree(project: &Project) -> Folder {
    let mut root = Folder::default();
    // Folders first, so the empty ones show too.
    for dir in &project.folders {
        root.at(project.relative(dir));
    }
    for path in &project.files {
        let parent = project.relative(path).parent().unwrap_or(Path::new(""));
        root.at(parent).files.push(path.clone());
    }
    root
}

/// "New file…" and "New folder…", creating in `dir`.
fn new_buttons(ui: &mut egui::Ui, dir: &Path, action: &mut Option<FileAction>) {
    if ui.button("New file…").clicked() {
        *action = Some(FileAction::NewFile(dir.to_path_buf()));
    }
    if ui.button("New folder…").clicked() {
        *action = Some(FileAction::NewFolder(dir.to_path_buf()));
    }
}

/// "Show folder": opens the file manager on `path` (a folder, or a file's folder).
fn show_folder_button(ui: &mut egui::Ui, path: &Path, action: &mut Option<FileAction>) {
    if ui.button("Show folder").clicked() {
        *action = Some(FileAction::ShowFolder(path.to_path_buf()));
    }
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
        let name = ui.strong(project.name());
        drop_target(ui, &name, &project.root, &mut action);
        if ui.small_button("+").on_hover_text("New file").clicked() {
            action = Some(FileAction::NewFile(project.root.clone()));
        }
        if ui.small_button("🗀").on_hover_text("New folder").clicked() {
            action = Some(FileAction::NewFolder(project.root.clone()));
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
        drop_target(ui, &rest, &project.root, &mut action);
        rest.context_menu(|ui| {
            new_buttons(ui, &project.root, &mut action);
            show_folder_button(ui, &project.root, &mut action);
        });
    });
    drag_preview(ui);
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
        let response = drag_source(header.header_response, &sub_dir);
        drop_target(ui, &response, &sub_dir, action);
        response.context_menu(|ui| {
            new_buttons(ui, &sub_dir, action);
            show_folder_button(ui, &sub_dir, action);
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
        // The entry file stays at the project's root.
        let response = if entry {
            response.on_hover_text(if project.language == crate::project::Language::C {
                "Entry file: holds main (every .c and .has file is built)"
            } else {
                "Entry file: the assembler starts here"
            })
        } else {
            drag_source(response, path)
        };
        drop_target(ui, &response, path.parent().unwrap_or(dir), action);
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
            new_buttons(ui, path.parent().unwrap_or(dir), action);
            show_folder_button(ui, path, action);
        });
    }
}
