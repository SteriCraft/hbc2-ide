//! The find bar, at the bottom of the editor area: find in the current file
//! (Ctrl+F) or in the project's files (Ctrl+Shift+F).

use eframe::egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use eframe::egui::text_edit::TextEditState;
use eframe::egui::{self, Align, Color32, FontId, Id, Key, Layout, RichText, TextEdit};

use crate::app::{Action, State};
use crate::document::Document;
use crate::find::{self, FilesResult, Find, FindMode};
use crate::views::editor::editor_id;

/// Most matches a "find in files" search lists.
const MAX_HITS: usize = 2000;

fn query_id() -> Id {
    Id::new("find-query")
}

/// Shows the find bar at the bottom of `ui`, when it is open. `doc` is the
/// document of the editor tab the bar is shown in.
pub fn find_bar(ui: &mut egui::Ui, state: &mut State, doc: Option<usize>) {
    let Some(mode) = state.find.mode else {
        return;
    };
    let State {
        find,
        docs,
        actions,
        project,
        ..
    } = state;
    egui::Panel::bottom("find-bar")
        .resizable(false)
        .show(ui, |ui| {
            ui.add_space(2.0);
            let closing = match mode {
                FindMode::File => doc.is_some_and(|doc| find_in_file(ui, find, &mut docs[doc])),
                FindMode::Files => {
                    let (search, closing) = files_row(ui, find);
                    if let Some(search) = search {
                        let files = project.as_ref().map_or(&[][..], |p| &p.files[..]);
                        find.results = Some(search_files(files, docs, &search, find.options));
                    }
                    if let Some(results) = &find.results {
                        results_list(ui, results, project.as_ref(), actions);
                    }
                    closing
                }
            };
            if closing {
                actions.push(Action::CloseFind);
            }
            ui.add_space(2.0);
        });
}

/// Match case and whole word buttons. Returns true if one changed.
fn option_buttons(ui: &mut egui::Ui, find: &mut Find) -> bool {
    let case = ui
        .toggle_value(&mut find.options.case_sensitive, "Aa")
        .on_hover_text("Match case");
    let word = ui
        .toggle_value(&mut find.options.whole_word, "Word")
        .on_hover_text("Whole words only");
    case.changed() || word.changed()
}

/// What happened in the query field.
#[derive(Default)]
struct Query {
    changed: bool,
    /// Enter (with Shift held or not).
    enter: Option<bool>,
    escape: bool,
}

/// The query field, filling the width left.
fn query_field(ui: &mut egui::Ui, find: &mut Find, hint: &str) -> Query {
    let mut output = TextEdit::singleline(&mut find.query)
        .id(query_id())
        .hint_text(hint.to_string())
        .font(egui::TextStyle::Monospace)
        .desired_width(ui.available_width())
        .show(ui);
    let response = &output.response;
    let (enter, shift, escape) = ui.input(|i| {
        (
            i.key_pressed(Key::Enter),
            i.modifiers.shift,
            i.key_pressed(Key::Escape),
        )
    });
    let mut query = Query {
        changed: response.changed(),
        ..Query::default()
    };
    if response.lost_focus() || response.has_focus() {
        if enter {
            query.enter = Some(shift);
            response.request_focus(); // keep typing after Enter
        } else if escape {
            query.escape = true;
        }
    }
    if std::mem::take(&mut find.focus_query) {
        response.request_focus();
        let all = CCursorRange::two(CCursor::new(0), CCursor::new(find.query.chars().count()));
        output.state.cursor.set_char_range(Some(all));
        output.state.store(ui.ctx(), query_id());
    }
    query
}

/// A drawn cross: the default fonts have no cross character.
fn close_button(ui: &mut egui::Ui) -> bool {
    let size = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
    }
    let cross = rect.shrink(size * 0.3);
    let stroke = egui::Stroke::new(1.5, visuals.fg_stroke.color);
    ui.painter()
        .line_segment([cross.left_top(), cross.right_bottom()], stroke);
    ui.painter()
        .line_segment([cross.right_top(), cross.left_bottom()], stroke);
    response.on_hover_text("Close (Escape)").clicked()
}

/// The bar for one file. Returns true when it is to close.
fn find_in_file(ui: &mut egui::Ui, find: &mut Find, doc: &mut Document) -> bool {
    let id = editor_id(&doc.path);
    let mut matches = find::find_all(&doc.text, &find.query, find.options);
    // Searches go on from the current match. The editor cannot show it as
    // its selection while the find bar has the keyboard (an editor without
    // focus has no selection), so the bar keeps it. Once the editor has the
    // keyboard again, its cursor is where searches go on from.
    if ui.memory(|m| m.has_focus(id)) {
        find.current = None;
    }
    let current = find
        .current
        .as_ref()
        .filter(|(path, m)| *path == doc.path && matches.contains(m))
        .map(|(_, m)| m.clone());
    let selection = current.clone().unwrap_or_else(|| {
        TextEditState::load(ui.ctx(), id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| r.as_sorted_char_range())
            .map_or(0..0, |r| r.start.0..r.end.0)
    });
    let mut go = None;
    let mut closing = false;
    ui.horizontal(|ui| {
        let options_changed = option_buttons(ui, find);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            closing |= close_button(ui);
            if ui.button("Next").on_hover_text("Enter").clicked() {
                go = find::next_match(&matches, selection.end);
            }
            if ui.button("Previous").on_hover_text("Shift+Enter").clicked() {
                go = find::previous_match(&matches, selection.start);
            }
            let count = if find.query.is_empty() {
                RichText::new("")
            } else if matches.is_empty() {
                RichText::new("No matches").color(ui.visuals().error_fg_color)
            } else {
                match matches.iter().position(|m| Some(m) == current.as_ref()) {
                    Some(i) => RichText::new(format!("{} of {}", i + 1, matches.len())),
                    None => RichText::new(format!("{} matches", matches.len())),
                }
            };
            ui.label(count);
            let query = query_field(ui, find, "Find");
            if query.changed || options_changed {
                // Search as you type, from the start of the selection.
                matches = find::find_all(&doc.text, &find.query, find.options);
                go = find::next_match(&matches, selection.start);
            }
            match query.enter {
                Some(false) => go = find::next_match(&matches, selection.end),
                Some(true) => go = find::previous_match(&matches, selection.start),
                None => {}
            }
            closing |= query.escape;
        });
    });
    if let Some(index) = go {
        find.current = Some((doc.path.clone(), matches[index].clone()));
        doc.select = Some(matches[index].clone()); // brings it into view
    }
    closing
}

/// The query row of "find in files". Returns the text to search for when
/// a search starts, and true when the bar is to close.
fn files_row(ui: &mut egui::Ui, find: &mut Find) -> (Option<String>, bool) {
    let mut search = false;
    let mut closing = false;
    ui.horizontal(|ui| {
        option_buttons(ui, find);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            closing |= close_button(ui);
            search |= ui
                .button("Find")
                .on_hover_text("Search every source file of the project (Enter)")
                .clicked();
            let query = query_field(ui, find, "Find in the project's files");
            search |= query.enter.is_some();
            closing |= query.escape;
        });
    });
    let search = (search && !find.query.is_empty()).then(|| find.query.clone());
    (search, closing)
}

/// Searches the project's files; open files are searched as edited.
fn search_files(
    files: &[std::path::PathBuf],
    docs: &[Document],
    query: &str,
    options: find::Options,
) -> FilesResult {
    let mut result = FilesResult {
        query: query.to_string(),
        ..FilesResult::default()
    };
    for path in files {
        let hits = match docs.iter().find(|d| &d.path == path) {
            Some(doc) => find::find_in_text(path, &doc.text, query, options),
            None => match std::fs::read_to_string(path) {
                Ok(text) => find::find_in_text(path, &text, query, options),
                Err(_) => continue,
            },
        };
        if !hits.is_empty() {
            result.files += 1;
            result.hits.extend(hits);
        }
    }
    result
}

fn results_list(
    ui: &mut egui::Ui,
    results: &FilesResult,
    project: Option<&crate::project::Project>,
    actions: &mut Vec<Action>,
) {
    let count = results.hits.len();
    let summary = match count {
        0 => format!("No matches for \"{}\".", results.query),
        _ => format!(
            "{count} match{} in {} file{} for \"{}\"{}.",
            if count == 1 { "" } else { "es" },
            results.files,
            if results.files == 1 { "" } else { "s" },
            results.query,
            if count > MAX_HITS {
                format!(" (showing the first {MAX_HITS})")
            } else {
                String::new()
            }
        ),
    };
    ui.label(RichText::new(summary).weak());
    if count == 0 {
        return;
    }
    let font = FontId::monospace(egui::TextStyle::Monospace.resolve(ui.style()).size);
    let text_color = ui.visuals().text_color();
    let highlight = Color32::from_rgba_unmultiplied(80, 140, 255, 90);
    egui::ScrollArea::vertical()
        .id_salt("find-results")
        .max_height(180.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let mut last_path = None;
            for hit in results.hits.iter().take(MAX_HITS) {
                if last_path != Some(&hit.path) {
                    last_path = Some(&hit.path);
                    let name = match project {
                        Some(p) => p.relative(&hit.path).display().to_string(),
                        None => hit.path.display().to_string(),
                    };
                    ui.label(RichText::new(name).strong());
                }
                let job = hit_layout(hit, font.clone(), text_color, highlight);
                let response = ui
                    .add(egui::Button::selectable(false, job).frame_when_inactive(false))
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if response.clicked() {
                    actions.push(Action::ShowMatch {
                        path: hit.path.clone(),
                        line: hit.line,
                        columns: hit.columns.clone(),
                    });
                }
            }
        });
}

/// "  42: the line", with the match highlighted and leading blanks dropped.
fn hit_layout(hit: &find::Hit, font: FontId, color: Color32, highlight: Color32) -> LayoutJob {
    let line = &hit.text;
    let lead = line.chars().take_while(|c| c.is_whitespace()).count();
    let byte = |chars: usize| {
        line.char_indices()
            .nth(chars)
            .map_or(line.len(), |(i, _)| i)
    };
    let (start, from, to) = (byte(lead), byte(hit.columns.start), byte(hit.columns.end));
    let plain = TextFormat::simple(font.clone(), color);
    let marked = TextFormat {
        background: highlight,
        ..plain.clone()
    };
    let weak = TextFormat::simple(font, color.gamma_multiply(0.6));
    let mut job = LayoutJob::default();
    job.append(&format!("{:>5}: ", hit.line), 0.0, weak);
    job.append(&line[start.min(from)..from], 0.0, plain.clone());
    job.append(&line[from..to], 0.0, marked);
    job.append(&line[to..], 0.0, plain);
    job
}
