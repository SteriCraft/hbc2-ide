//! Sections laid out in fixed columns, centered.
//!
//! A column never gets narrower than the widest content it has shown, so the
//! layout does not shift while the emulator runs.

use eframe::egui::{self, Id, TextWrapMode};

/// Space between two columns.
const COLUMN_GAP: f32 = 32.0;
/// Space between two sections of a column.
const SECTION_GAP: f32 = 12.0;

pub type Section<'a> = Box<dyn FnOnce(&mut egui::Ui) + 'a>;

/// Shows `columns` of sections side by side, centered. `id` must be unique
/// to the view. Returns the width they need.
pub fn columns(ui: &mut egui::Ui, id: &str, columns: Vec<Vec<Section<'_>>>) -> f32 {
    let id = Id::new(("columns", id));
    let count = columns.len();
    let mut widths: Vec<f32> = ui
        .data(|d| d.get_temp::<Vec<f32>>(id))
        .filter(|widths| widths.len() == count)
        .unwrap_or_else(|| vec![0.0; count]);
    let before = widths.clone();
    let margin = ((ui.available_width() - total(&widths)) / 2.0).max(0.0);

    let spacing = ui.spacing().item_spacing;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(margin);
        for (index, sections) in columns.into_iter().enumerate() {
            if index > 0 {
                ui.add_space(COLUMN_GAP);
            }
            let width = ui
                .vertical(|ui| {
                    ui.spacing_mut().item_spacing = spacing;
                    // No wrapping: the column takes the width of its content.
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    ui.set_min_width(widths[index]);
                    for (n, section) in sections.into_iter().enumerate() {
                        if n > 0 {
                            ui.add_space(SECTION_GAP);
                        }
                        section(ui);
                    }
                })
                .response
                .rect
                .width();
            // Ignore rounding noise, or every frame would be laid out twice.
            if width > widths[index] + 0.5 {
                widths[index] = width;
            }
        }
    });

    if widths != before {
        ui.data_mut(|d| d.insert_temp(id, widths.clone()));
        // Lay out again with the new widths, so nothing jumps on screen.
        ui.ctx().request_discard("column widths changed");
    }
    total(&widths)
}

fn total(widths: &[f32]) -> f32 {
    widths.iter().sum::<f32>() + widths.len().saturating_sub(1) as f32 * COLUMN_GAP
}

#[cfg(test)]
mod tests {
    #[test]
    fn total_width_counts_the_gaps() {
        assert_eq!(super::total(&[]), 0.0);
        assert_eq!(super::total(&[100.0]), 100.0);
        assert_eq!(super::total(&[100.0, 50.0]), 150.0 + super::COLUMN_GAP);
    }
}
