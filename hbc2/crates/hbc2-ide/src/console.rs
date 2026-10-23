//! Messages from the IDE, the assembler and the emulator.

use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub level: Level,
    pub text: String,
    /// Source location to open when the message is clicked.
    pub location: Option<(PathBuf, u32)>,
}

#[derive(Default)]
pub struct Console {
    pub messages: Vec<Message>,
}

const MAX_MESSAGES: usize = 2000;

impl Console {
    pub fn push(
        &mut self,
        level: Level,
        text: impl Into<String>,
        location: Option<(PathBuf, u32)>,
    ) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.drain(..MAX_MESSAGES / 2);
        }
        self.messages.push(Message {
            level,
            text: text.into(),
            location,
        });
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(Level::Info, text, None);
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(Level::Success, text, None);
    }

    pub fn warning(&mut self, text: impl Into<String>) {
        self.push(Level::Warning, text, None);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(Level::Error, text, None);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Shows the messages. Returns the location of a clicked message.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<(PathBuf, u32)> {
        let mut clicked = None;
        ui.horizontal(|ui| {
            if ui.small_button("Clear").clicked() {
                self.clear();
            }
            let errors = self.count(Level::Error);
            let warnings = self.count(Level::Warning);
            ui.label(format!("{errors} error(s), {warnings} warning(s)"));
        });
        ui.separator();
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let dark = ui.visuals().dark_mode;
        egui::ScrollArea::both()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show_rows(ui, row_height, self.messages.len(), |ui, rows| {
                for message in &self.messages[rows] {
                    let color = level_color(message.level, dark);
                    let text = RichText::new(&message.text).monospace().color(color);
                    if message.location.is_some() {
                        let response = ui
                            .add(egui::Label::new(text).sense(egui::Sense::click()))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Click to show in the editor");
                        if response.clicked() {
                            clicked = message.location.clone();
                        }
                    } else {
                        ui.label(text);
                    }
                }
            });
        clicked
    }

    fn count(&self, level: Level) -> usize {
        self.messages.iter().filter(|m| m.level == level).count()
    }
}

pub fn level_color(level: Level, dark: bool) -> Color32 {
    match (level, dark) {
        (Level::Info, true) => Color32::from_gray(200),
        (Level::Info, false) => Color32::from_gray(40),
        (Level::Success, true) => Color32::from_rgb(120, 200, 120),
        (Level::Success, false) => Color32::from_rgb(0, 120, 0),
        (Level::Warning, true) => Color32::from_rgb(230, 190, 80),
        (Level::Warning, false) => Color32::from_rgb(160, 100, 0),
        (Level::Error, true) => Color32::from_rgb(240, 100, 100),
        (Level::Error, false) => Color32::from_rgb(190, 0, 0),
    }
}
