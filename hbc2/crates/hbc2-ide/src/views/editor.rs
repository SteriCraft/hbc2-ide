//! The source code editor: line numbers, breakpoints, highlighting, and the
//! current instruction and errors marked on their lines.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use eframe::egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use eframe::egui::text_edit::TextEditState;
use eframe::egui::{
    self, Align, Align2, Color32, Event, FontId, Frame, Galley, Id, Key, Margin, Modifiers, Rect,
    Sense, Shape, Stroke, TextEdit, pos2, vec2,
};

use crate::document::{Document, shift_breakpoints};
use crate::editing;
use crate::find::{self, Options};
use crate::highlight;
use crate::signatures::{Signature, Signatures};

/// What the editor shows besides the text.
pub struct Marks<'a> {
    pub breakpoints: &'a mut BTreeSet<u32>,
    /// Line of the instruction the CPU will run next.
    pub pc_line: Option<u32>,
    /// The program is running: the text cannot be edited.
    pub read_only: bool,
    /// Errors and warnings: (line, is_error, message).
    pub diagnostics: &'a [(u32, bool, String)],
    /// Lines holding an instruction in the last build (`None` = no build).
    pub code_lines: Option<&'a BTreeSet<u32>>,
    /// Text of the find bar, whose matches are highlighted.
    pub search: Option<(&'a str, Options)>,
    /// The find bar's current match (else the selection, if it is a match).
    pub current_match: Option<std::ops::Range<usize>>,
    /// Known C functions: their parameters are shown while typing a call.
    pub signatures: Option<&'a Signatures>,
    pub font_size: f32,
}

#[derive(Default)]
pub struct EditorResponse {
    pub changed: bool,
    pub breakpoints_changed: bool,
}

const MARKER_WIDTH: f32 = 16.0;

/// Id of the text widget of the editor of `path` (its state holds the
/// cursor and the undo history).
pub fn editor_id(path: &Path) -> Id {
    Id::new(("editor", path))
}

pub fn editor_ui(ui: &mut egui::Ui, doc: &mut Document, marks: Marks<'_>) -> EditorResponse {
    let mut result = EditorResponse::default();
    let font = FontId::monospace(marks.font_size);
    let dark = ui.visuals().dark_mode;
    let digit_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    let line_total = crate::document::line_count(&doc.text);
    let digits = line_total.to_string().len().max(3) as f32;
    let gutter_width = (MARKER_WIDTH + digits * digit_width + 10.0).min(127.0);
    let editor_id = editor_id(&doc.path);
    let syntax = highlight::Syntax::of(&doc.path);

    let Document {
        text,
        layout_cache,
        scroll_to,
        scroll_gently,
        select,
        focus,
        ..
    } = doc;

    egui::ScrollArea::both()
        .id_salt(("editor-scroll", editor_id))
        .auto_shrink(false)
        .show(ui, |ui| {
            // Placeholder painted under the text, filled once the layout is known.
            let background = ui.painter().add(Shape::Noop);

            let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
                let source = buffer.as_str();
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                (source, dark, marks.font_size.to_bits()).hash(&mut hasher);
                let key = hasher.finish();
                let job = match layout_cache {
                    Some((k, job)) if *k == key => job.clone(),
                    _ => {
                        let mut job = highlight::layout_job(source, syntax, font.clone(), dark);
                        job.wrap.max_width = f32::INFINITY;
                        *layout_cache = Some((key, job.clone()));
                        job
                    }
                };
                ui.fonts_mut(|f| f.layout_job(job))
            };

            // Fill the tab even when the file is short (the background is the editor).
            let row_height = ui.fonts_mut(|f| f.row_height(&font));
            let visible_rows = ((ui.available_height() - 6.0) / row_height)
                .floor()
                .max(1.0) as usize;
            let focused = ui.memory(|m| m.has_focus(editor_id));
            let before = focused.then(|| text.clone());
            let auto_edited = focused && !marks.read_only && auto_edit(ui, editor_id, text);
            // A `&str` buffer can be selected and copied, not edited.
            let mut shown: &str;
            let buffer: &mut dyn egui::TextBuffer = if marks.read_only {
                shown = text.as_str();
                &mut shown
            } else {
                text
            };
            let mut output = TextEdit::multiline(buffer)
                .id(editor_id)
                .font(font.clone())
                .code_editor()
                .lock_focus(true)
                .desired_width(f32::INFINITY)
                .desired_rows((line_total as usize).max(visible_rows))
                .frame(Frame::NONE.inner_margin(Margin {
                    left: gutter_width as i8,
                    right: 8,
                    top: 2,
                    bottom: 2,
                }))
                .layouter(&mut layouter)
                .show(ui);

            if std::mem::take(focus) {
                output.response.request_focus();
            }
            if output.response.dragged() {
                drag_scroll(ui);
            }
            if output.response.clicked()
                && let Some(pointer) = output.response.interact_pointer_pos()
                && click_count(ui, editor_id, pointer) < 3
                && output.response.triple_clicked()
            {
                // egui took this double click for a triple one: select the word.
                let at = output
                    .galley
                    .cursor_from_pos(pointer - output.galley_pos)
                    .index
                    .0;
                let (start, end) = word_at(text, at);
                output.state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(start),
                    CCursor::new(end),
                )));
                output.state.clone().store(ui.ctx(), editor_id);
            }
            if auto_edited {
                // The text box did not see the change: bring the cursor into view.
                if let Some(range) = output.state.cursor.char_range() {
                    let at = range.primary.index.0;
                    let rect = text_rect(&output.galley, output.galley_pos, &(at..at));
                    ui.scroll_to_rect(rect.expand2(vec2(8.0, 0.0)), None);
                }
            }
            if output.response.changed() || auto_edited {
                result.changed = true;
                if let Some(before) = before {
                    let count = marks.breakpoints.len();
                    let old: Vec<u32> = marks.breakpoints.iter().copied().collect();
                    shift_breakpoints(marks.breakpoints, &before, text);
                    result.breakpoints_changed = count != marks.breakpoints.len()
                        || !marks.breakpoints.iter().eq(old.iter());
                }
            }

            let rows = line_rows(&output.galley, output.galley_pos);
            let full = output.response.rect;
            let gutter = Rect::from_min_max(full.min, pos2(full.min.x + gutter_width, full.max.y));
            let row_rect = |line: u32| {
                rows.get(line.wrapping_sub(1) as usize)
                    .map(|&(top, bottom)| Rect::from_x_y_ranges(full.x_range(), top..=bottom))
            };

            // Backgrounds: editor, current instruction, error lines.
            let visuals = ui.visuals();
            let mut shapes = vec![Shape::rect_filled(full, 0.0, visuals.extreme_bg_color)];
            shapes.push(Shape::rect_filled(gutter, 0.0, visuals.faint_bg_color));
            for (line, is_error, _) in marks.diagnostics {
                if let Some(rect) = row_rect(*line) {
                    let color = if *is_error {
                        Color32::from_rgba_unmultiplied(255, 60, 60, 40)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 190, 0, 30)
                    };
                    shapes.push(Shape::rect_filled(rect, 0.0, color));
                }
            }
            if let Some(rect) = marks.pc_line.and_then(row_rect) {
                shapes.push(Shape::rect_filled(rect, 0.0, pc_color(dark)));
            }
            // Find matches; the selected one stands out.
            if let Some((query, options)) = marks.search {
                let selected = marks.current_match.clone().or_else(|| {
                    output
                        .state
                        .cursor
                        .char_range()
                        .map(|r| r.as_sorted_char_range())
                        .map(|r| r.start.0..r.end.0)
                });
                let clip = ui.clip_rect();
                for m in find::find_all(text, query, options) {
                    let rect = text_rect(&output.galley, output.galley_pos, &m);
                    if !clip.intersects(rect) {
                        continue;
                    }
                    let current = selected.as_ref() == Some(&m);
                    let (fill, stroke) = match_colors(dark, current);
                    shapes.push(Shape::rect_filled(rect, 2.0, fill));
                    if current {
                        shapes.push(Shape::rect_stroke(
                            rect,
                            2.0,
                            Stroke::new(1.0, stroke),
                            egui::StrokeKind::Outside,
                        ));
                    }
                }
            }
            ui.painter().set(background, Shape::Vec(shapes));

            // Gutter: breakpoints, PC arrow, line numbers.
            let painter = ui.painter();
            let number_color = visuals.weak_text_color();
            for (index, &(top, bottom)) in rows.iter().enumerate() {
                let line = index as u32 + 1;
                let y = (top + bottom) / 2.0;
                let marker = pos2(gutter.min.x + MARKER_WIDTH / 2.0, y);
                if marks.breakpoints.contains(&line) {
                    let bound = marks.code_lines.is_none_or(|lines| lines.contains(&line));
                    let red = Color32::from_rgb(220, 50, 50);
                    if bound {
                        painter.circle_filled(marker, 5.0, red);
                    } else {
                        painter.circle_stroke(marker, 4.5, Stroke::new(1.5, red));
                    }
                }
                if let Some((_, is_error, _)) = marks.diagnostics.iter().find(|d| d.0 == line) {
                    let color = if *is_error {
                        Color32::from_rgb(240, 70, 70)
                    } else {
                        Color32::from_rgb(230, 170, 0)
                    };
                    painter.rect_filled(
                        Rect::from_center_size(
                            pos2(gutter.max.x - 3.0, y),
                            vec2(3.0, bottom - top),
                        ),
                        0.0,
                        color,
                    );
                }
                if marks.pc_line == Some(line) {
                    painter.text(
                        marker,
                        Align2::CENTER_CENTER,
                        "▶",
                        FontId::proportional(marks.font_size * 0.8),
                        Color32::from_rgb(240, 200, 40),
                    );
                }
                painter.text(
                    pos2(gutter.max.x - 8.0, y),
                    Align2::RIGHT_CENTER,
                    line.to_string(),
                    font.clone(),
                    number_color,
                );
            }

            // Click in the gutter: toggle a breakpoint. Hover: show the line's diagnostics.
            let gutter_response = ui
                .interact(gutter, editor_id.with("gutter"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let pointer_line = gutter_response.hover_pos().and_then(|p| {
                rows.iter()
                    .position(|&(top, bottom)| (top..=bottom).contains(&p.y))
                    .map(|i| i as u32 + 1)
            });
            if gutter_response.clicked()
                && let Some(line) = pointer_line
            {
                if !marks.breakpoints.remove(&line) {
                    marks.breakpoints.insert(line);
                }
                result.breakpoints_changed = true;
            }
            if let Some(line) = pointer_line {
                let messages: Vec<&str> = marks
                    .diagnostics
                    .iter()
                    .filter(|d| d.0 == line)
                    .map(|d| d.2.as_str())
                    .collect();
                if !messages.is_empty() {
                    gutter_response.on_hover_ui_at_pointer(|ui| {
                        for message in messages {
                            ui.label(message);
                        }
                    });
                }
            }

            // Inside the brackets of a known function's call: its parameters.
            if let Some(signatures) = marks.signatures
                && output.response.has_focus()
                && let Some(range) = output.state.cursor.char_range()
                && range.is_empty()
            {
                let at = range.primary.index.0;
                if let Some((call, signature)) = signatures.call_at(text, at) {
                    let cursor = text_rect(&output.galley, output.galley_pos, &(at..at));
                    signature_popup(ui, editor_id, signature, call.arg, cursor, marks.font_size);
                }
            }

            // Go to a line: scroll there and put the cursor at its start. Only
            // the start of the row is brought into view, so a long line does
            // not scroll the view to the right.
            if let Some(line) = scroll_to.take()
                && let Some(rect) = row_rect(line)
            {
                let rect = rect.with_max_x(rect.min.x + 1.0);
                let align = (!std::mem::take(scroll_gently)).then_some(Align::Center);
                ui.scroll_to_rect(rect.expand2(vec2(0.0, rect.height() * 2.0)), align);
                let index = char_index_of_line(text, line);
                output
                    .state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(index))));
                output.state.clone().store(ui.ctx(), editor_id);
            }

            // Select some text (a find match, an undone edit) and scroll to it.
            if let Some(range) = select.take() {
                let rect = text_rect(&output.galley, output.galley_pos, &range);
                ui.scroll_to_rect(rect.expand2(vec2(48.0, rect.height() * 2.0)), None);
                let selected = output.state.cursor.char_range().map(|r| {
                    let r = r.as_sorted_char_range();
                    r.start.0..r.end.0
                });
                if selected != Some(range.clone()) {
                    output.state.cursor.set_char_range(Some(CCursorRange::two(
                        CCursor::new(range.start),
                        CCursor::new(range.end),
                    )));
                    output.state.clone().store(ui.ctx(), editor_id);
                }
            }
        });
    result
}

/// Keys the editor handles before the text box (see [`editing`]): Tab on
/// several lines, Shift+Tab, brackets, Backspace between brackets, Enter.
/// They are taken from the input in the order they came, up to the first
/// key left to the text box. Returns whether the text changed.
fn auto_edit(ui: &egui::Ui, id: Id, text: &mut String) -> bool {
    let Some(mut state) = TextEditState::load(ui.ctx(), id) else {
        return false;
    };
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    let (mut anchor, mut cursor) = (range.secondary.index.0, range.primary.index.0);
    let events = ui.input(|i| i.events.clone());
    let mut taken = Vec::new();
    let mut changed = false;
    for (index, event) in events.iter().enumerate() {
        let edit = match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => match (key, modifiers) {
                (Key::Tab, m) if m.is_none() && editing::spans_lines(text, anchor, cursor) => {
                    Some(editing::indent(text, anchor, cursor, false))
                }
                (Key::Tab, m) if m.matches_exact(Modifiers::SHIFT) => {
                    Some(editing::indent(text, anchor, cursor, true))
                }
                (Key::Enter, m) if m.is_none() => Some(editing::newline(text, anchor, cursor)),
                (Key::Backspace, m) if m.is_none() => editing::backspace(text, anchor, cursor),
                // Keys the text box acts on: editing, moving the cursor, Ctrl
                // shortcuts (AltGr is Ctrl+Alt on Windows: it types characters).
                (key, m) if edits_or_moves(*key) || (m.command && !m.alt) => None,
                // A character key: its character comes in a Text event.
                _ => continue,
            },
            Event::Text(typed) => {
                let mut typed_chars = typed.chars();
                match (typed_chars.next(), typed_chars.next()) {
                    (Some(c), None) => editing::open_bracket(text, anchor, cursor, c)
                        .or_else(|| editing::close_bracket(text, anchor, cursor, c))
                        .or_else(|| editing::close_brace(text, anchor, cursor, c)),
                    _ => None,
                }
            }
            // Pointer moves, key releases...: no change to the text.
            Event::Paste(_) | Event::Cut | Event::Ime(_) => None,
            _ => continue,
        };
        let Some(edit) = edit else {
            break; // the text box handles this one, and the ones after it
        };
        taken.push(index);
        changed |= edit.text != *text;
        *text = edit.text;
        (anchor, cursor) = (edit.anchor, edit.cursor);
    }
    if taken.is_empty() {
        return false;
    }
    ui.ctx().input_mut(|i| {
        let mut index = 0;
        i.events.retain(|_| {
            index += 1;
            !taken.contains(&(index - 1))
        });
    });
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(anchor),
        CCursor::new(cursor),
    )));
    state.store(ui.ctx(), id);
    changed
}

/// Counts the clicks in a row on the editor, at the same place, each one
/// soon after the one before: 1, 2 (double click), 3 (triple click)...
/// egui takes a double click that comes soon after another click for a
/// triple click (it does not check that the clicks before were a double
/// click), which selected the line instead of the word.
fn click_count(ui: &egui::Ui, id: Id, pointer: egui::Pos2) -> u32 {
    let id = id.with("clicks");
    let now = ui.input(|i| i.time);
    let (delay, distance) = ui.ctx().options(|o| {
        (
            o.input_options.max_double_click_delay,
            o.input_options.max_click_dist,
        )
    });
    let last: Option<(f64, egui::Pos2, u32)> = ui.data(|d| d.get_temp(id));
    let count = match last {
        Some((time, position, count))
            if now - time < delay && position.distance(pointer) < distance =>
        {
            count + 1
        }
        _ => 1,
    };
    ui.data_mut(|d| d.insert_temp(id, (now, pointer, count)));
    count
}

/// The word (letters, digits, `_`) at the character position `at`, or
/// just before it (a click on the right half of its last letter); an
/// empty range elsewhere.
fn word_at(text: &str, at: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let is_word = |i: usize| {
        chars
            .get(i)
            .is_some_and(|&c| c.is_alphanumeric() || c == '_')
    };
    let at = if is_word(at) || at == 0 || !is_word(at - 1) {
        at
    } else {
        at - 1
    };
    if !is_word(at) {
        return (at, at);
    }
    let start = (0..at)
        .rev()
        .take_while(|&i| is_word(i))
        .last()
        .unwrap_or(at);
    let end = (at..chars.len())
        .find(|&i| !is_word(i))
        .unwrap_or(chars.len());
    (start, end)
}

/// The box under the cursor with the signature of the function being
/// called, the parameter of the argument typed (`arg`) stressed, and the
/// function's comment.
fn signature_popup(
    ui: &egui::Ui,
    id: Id,
    signature: &Signature,
    arg: usize,
    cursor: Rect,
    font_size: f32,
) {
    const WIDTH: f32 = 520.0;
    let visuals = ui.visuals();
    let font = FontId::monospace(font_size * 0.9);
    let plain = TextFormat::simple(font.clone(), visuals.text_color());
    let weak = TextFormat::simple(font.clone(), visuals.weak_text_color());
    let current = TextFormat {
        color: visuals.strong_text_color(),
        background: visuals.selection.bg_fill.gamma_multiply(0.6),
        ..TextFormat::simple(font, visuals.strong_text_color())
    };

    let mut job = LayoutJob::default();
    job.wrap.max_width = WIDTH;
    let head = if signature.is_macro() {
        format!("#define {}(", signature.name)
    } else if signature.returns.ends_with('*') {
        format!("{}{}(", signature.returns, signature.name)
    } else {
        format!("{} {}(", signature.returns, signature.name)
    };
    job.append(&head, 0.0, plain.clone());
    if signature.params.is_empty() && !signature.is_macro() {
        job.append("void", 0.0, weak.clone());
    }
    let stressed = signature.param_for(arg);
    for (index, param) in signature.params.iter().enumerate() {
        if index > 0 {
            job.append(", ", 0.0, plain.clone());
        }
        let format = if stressed == Some(index) {
            &current
        } else {
            &plain
        };
        job.append(param, 0.0, format.clone());
    }
    job.append(")", 0.0, plain);

    // Below the cursor's row, or above it near the bottom of the view.
    let below = cursor.max.y + 160.0 < ui.clip_rect().bottom();
    let (position, pivot) = if below {
        (pos2(cursor.min.x, cursor.max.y + 4.0), Align2::LEFT_TOP)
    } else {
        (pos2(cursor.min.x, cursor.min.y - 4.0), Align2::LEFT_BOTTOM)
    };
    egui::Area::new(id.with("signature"))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .fixed_pos(position)
        .pivot(pivot)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(WIDTH);
                ui.label(job);
                if !signature.doc.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&signature.doc)
                            .size(font_size * 0.85)
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            });
        });
}

/// Keys that change the text or move the cursor in the text box.
fn edits_or_moves(key: Key) -> bool {
    matches!(
        key,
        Key::Backspace
            | Key::Delete
            | Key::Enter
            | Key::Tab
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
    )
}

/// While text is selected with the mouse, a pointer past an edge of the
/// view scrolls toward it, faster the further it is. Called in the scroll
/// area, during the drag.
fn drag_scroll(ui: &egui::Ui) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let view = ui.clip_rect();
    let past = |p: f32, low: f32, high: f32| {
        if p < low {
            p - low
        } else if p > high {
            p - high
        } else {
            0.0
        }
    };
    let distance = vec2(
        past(pointer.x, view.left(), view.right()),
        past(pointer.y, view.top(), view.bottom()),
    );
    if distance == egui::Vec2::ZERO {
        return;
    }
    // Points per second: from a slow start at the edge to fast far away.
    let speed = |d: f32| {
        if d == 0.0 {
            0.0
        } else {
            d.signum() * (d.abs() * 12.0).clamp(150.0, 6000.0)
        }
    };
    let dt = ui.input(|i| i.stable_dt).min(0.05);
    // A positive delta moves the content down, which shows what is above.
    ui.scroll_with_delta(-vec2(speed(distance.x), speed(distance.y)) * dt);
    ui.ctx().request_repaint(); // keep scrolling while the mouse is still
}

/// Screen rectangle of some characters of one line.
fn text_rect(galley: &Galley, origin: egui::Pos2, chars: &std::ops::Range<usize>) -> Rect {
    let start = galley.pos_from_cursor(CCursor::new(chars.start));
    let end = galley.pos_from_cursor(CCursor::new(chars.end));
    Rect::from_min_max(start.min, pos2(end.min.x.max(start.min.x), start.max.y))
        .translate(origin.to_vec2())
}

/// (fill, outline) of a find match.
fn match_colors(dark: bool, current: bool) -> (Color32, Color32) {
    let alpha = match (dark, current) {
        (true, false) => 45,
        (true, true) => 110,
        (false, false) => 40,
        (false, true) => 90,
    };
    let fill = Color32::from_rgba_unmultiplied(80, 140, 255, alpha);
    (fill, Color32::from_rgb(80, 140, 255))
}

/// Screen y range (top, bottom) of each line of the text.
fn line_rows(galley: &Arc<Galley>, origin: egui::Pos2) -> Vec<(f32, f32)> {
    let mut rows = Vec::new();
    let mut line_start = true;
    for row in &galley.rows {
        let rect = row.rect().translate(origin.to_vec2());
        if line_start {
            rows.push((rect.min.y, rect.max.y));
        } else if let Some(last) = rows.last_mut() {
            last.1 = rect.max.y; // a wrapped line continues
        }
        line_start = row.ends_with_newline;
    }
    rows
}

pub fn char_index_of_line(text: &str, line: u32) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut seen = 1;
    for (index, c) in text.chars().enumerate() {
        if c == '\n' {
            seen += 1;
            if seen == line {
                return index + 1;
            }
        }
    }
    text.chars().count()
}

pub fn pc_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgba_unmultiplied(255, 210, 0, 45)
    } else {
        Color32::from_rgba_unmultiplied(255, 220, 0, 90)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts() {
        let text = "ab\ncé\n\nd";
        assert_eq!(char_index_of_line(text, 1), 0);
        assert_eq!(char_index_of_line(text, 2), 3);
        assert_eq!(char_index_of_line(text, 4), 7);
        assert_eq!(char_index_of_line(text, 9), 8);
    }

    #[test]
    fn words() {
        let text = "f(rand_range, x2);";
        assert_eq!(word_at(text, 4), (2, 12)); // inside
        assert_eq!(word_at(text, 2), (2, 12)); // its start
        assert_eq!(word_at(text, 12), (2, 12)); // just after its end
        assert_eq!(word_at(text, 15), (14, 16));
        assert_eq!(word_at(text, 13), (13, 13)); // on a space
        assert_eq!(word_at(text, 0), (0, 1));
        assert_eq!(word_at(text, 99), (99, 99));
    }
}
