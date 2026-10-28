//! Hex view of the RAM or the EEPROM. RAM bytes can be edited while paused.

use eframe::egui::{self, Color32, Event, Key, RichText, Sense, TextStyle};
use hbc2_core::isa::{INSTRUCTION_SIZE, IVT_SIZE, IVT_START, PROGRAM_START};
use hbc2_runner::{RunState, Runner, Snapshot};

use crate::views::cpu::changed_color;

const BYTES_PER_ROW: usize = 16;
/// Stack items shown under the hex view.
const STACK_ITEMS: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Ram,
    Eeprom,
}

pub struct MemoryView {
    source: Source,
    goto_text: String,
    /// Row to scroll to on the next frame.
    scroll_to_row: Option<usize>,
    /// RAM byte being edited, and the first hex digit typed.
    selected: Option<u16>,
    pending_digit: Option<u8>,
}

impl Default for MemoryView {
    fn default() -> Self {
        MemoryView {
            source: Source::Ram,
            goto_text: String::new(),
            scroll_to_row: None,
            selected: None,
            pending_digit: None,
        }
    }
}

impl MemoryView {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        s: &Snapshot,
        baseline: Option<&Snapshot>,
        runner: &Runner,
    ) {
        stack_strip(ui, s, baseline);
        let eeprom = s.eeprom.as_ref().map(|e| e.memory());
        if eeprom.is_none() {
            self.source = Source::Ram;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.source, Source::Ram, "RAM");
            ui.add_enabled_ui(eeprom.is_some(), |ui| {
                ui.selectable_value(&mut self.source, Source::Eeprom, "EEPROM")
                    .on_disabled_hover_text("No EEPROM plugged in");
            });
            ui.separator();
            ui.label("Go to");
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.goto_text)
                    .desired_width(70.0)
                    .hint_text("0300"),
            );
            let go = ui.small_button("Go").clicked()
                || (edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
            if go {
                let text = self
                    .goto_text
                    .trim()
                    .trim_start_matches('$')
                    .trim_start_matches("0x");
                if let Ok(address) = usize::from_str_radix(text, 16) {
                    self.scroll_to_row = Some(address / BYTES_PER_ROW);
                }
            }
            for (label, address) in [
                ("PC", s.cpu.pc as usize),
                ("Stack", 0),
                ("IVT", IVT_START as usize),
                ("Program", PROGRAM_START as usize),
            ] {
                if ui.small_button(label).clicked() {
                    self.source = Source::Ram;
                    self.scroll_to_row = Some(address / BYTES_PER_ROW);
                }
            }
        });

        let (memory, before): (&[u8], Option<&[u8]>) = match (self.source, eeprom) {
            (Source::Eeprom, Some(eeprom)) => (
                eeprom,
                baseline.and_then(|b| b.eeprom.as_ref()).map(|e| e.memory()),
            ),
            _ => (
                s.board.ram.as_slice(),
                baseline.map(|b| b.board.ram.as_slice()),
            ),
        };
        let editable = self.source == Source::Ram && s.state == RunState::Paused;
        if !editable {
            self.selected = None;
        }
        self.handle_typing(ui, runner);

        let legend = if editable {
            "Click a byte, then type hex digits to change it. Yellow: changed by the last run or step."
        } else {
            "Yellow: changed by the last run or step. Pause to edit RAM."
        };
        ui.label(RichText::new(legend).weak());
        ui.separator();

        let dark = ui.visuals().dark_mode;
        let row_height = ui.text_style_height(&TextStyle::Monospace) + 2.0;
        let rows = memory.len().div_ceil(BYTES_PER_ROW);
        let mut area = egui::ScrollArea::both().auto_shrink(false);
        if let Some(row) = self.scroll_to_row.take() {
            let spacing = ui.spacing().item_spacing.y;
            area = area.vertical_scroll_offset(row.min(rows - 1) as f32 * (row_height + spacing));
        }
        let pc = s.cpu.pc as usize;
        let sp = s.cpu.sp as usize;
        let address_digits = if memory.len() > 0x1_0000 { 5 } else { 4 };
        area.show_rows(ui, row_height, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let base = row * BYTES_PER_ROW;
                    ui.label(
                        RichText::new(format!("{base:0address_digits$X}"))
                            .monospace()
                            .weak(),
                    );
                    ui.add_space(6.0);
                    for i in 0..BYTES_PER_ROW {
                        let address = base + i;
                        let value = memory[address];
                        let mut text = RichText::new(format!("{value:02X}")).monospace();
                        let in_ram = self.source == Source::Ram;
                        if before.is_some_and(|b| b[address] != value) {
                            text = text.color(changed_color(dark)).strong();
                        }
                        if in_ram && (pc..pc + INSTRUCTION_SIZE as usize).contains(&address) {
                            text = text.background_color(crate::views::editor::pc_color(dark));
                        } else if in_ram && address == sp {
                            text = text.background_color(sp_color());
                        }
                        if in_ram && self.selected == Some(address as u16) {
                            text = text
                                .background_color(ui.visuals().selection.bg_fill)
                                .color(ui.visuals().selection.stroke.color);
                        }
                        let response = ui.add(egui::Label::new(text).sense(Sense::click()));
                        if response.clicked() && editable {
                            self.selected = Some(address as u16);
                            self.pending_digit = None;
                            if let Some(id) = ui.memory(|m| m.focused()) {
                                ui.memory_mut(|m| m.surrender_focus(id));
                            }
                        }
                        response.on_hover_text(describe(address, self.source));
                        if i == 7 {
                            ui.add_space(4.0);
                        }
                    }
                    ui.add_space(6.0);
                    let ascii: String = memory[base..base + BYTES_PER_ROW]
                        .iter()
                        .map(|&b| {
                            if (0x20..0x7F).contains(&b) {
                                b as char
                            } else {
                                '·'
                            }
                        })
                        .collect();
                    ui.label(RichText::new(ascii).monospace().weak());
                });
            }
        });
    }

    /// Hex digits typed while a byte is selected change it.
    fn handle_typing(&mut self, ui: &egui::Ui, runner: &Runner) {
        let Some(address) = self.selected else { return };
        if ui.memory(|m| m.focused().is_some()) {
            return; // typing goes to a text field or the monitor
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                Event::Text(text) => {
                    for digit in text.chars().filter_map(|c| c.to_digit(16)) {
                        let digit = digit as u8;
                        match self.pending_digit.take() {
                            None => self.pending_digit = Some(digit),
                            Some(high) => {
                                let value = high << 4 | digit;
                                runner.exec(move |m| m.board.ram.write(address, value));
                                self.selected = Some(address.wrapping_add(1));
                            }
                        }
                    }
                }
                Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    self.selected = None;
                    self.pending_digit = None;
                }
                _ => {}
            }
        }
    }
}

/// Background of the byte SP points at, here and in the hex view.
fn sp_color() -> Color32 {
    Color32::from_rgba_unmultiplied(80, 160, 255, 60)
}

/// The top of the stack, left to right, at the bottom of the tab.
fn stack_strip(ui: &mut egui::Ui, s: &Snapshot, baseline: Option<&Snapshot>) {
    let dark = ui.visuals().dark_mode;
    let sp = s.cpu.sp;
    egui::Panel::bottom("memory_stack")
        .resizable(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.strong("Stack")
                    .on_hover_text("Top of the stack first. SP points at the top item.");
                ui.label(RichText::new(format!("SP {sp:02X}, top item first")).weak());
            });
            egui::Grid::new("stack_strip")
                .spacing([12.0, 2.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Address").weak());
                    for depth in 0..STACK_ITEMS {
                        let address = sp.wrapping_sub(depth) as u16;
                        ui.label(RichText::new(format!("{address:04X}")).monospace().weak());
                    }
                    ui.end_row();
                    ui.label(RichText::new("Value").weak());
                    for depth in 0..STACK_ITEMS {
                        let address = sp.wrapping_sub(depth) as u16;
                        let value = s.board.ram.read(address);
                        let mut text = RichText::new(format!("{value:02X}")).monospace();
                        if baseline.is_some_and(|b| b.board.ram.read(address) != value) {
                            text = text.color(changed_color(dark)).strong();
                        }
                        if depth == 0 {
                            text = text.background_color(sp_color());
                        }
                        ui.label(text);
                    }
                    ui.end_row();
                });
            ui.add_space(4.0);
        });
}

fn describe(address: usize, source: Source) -> String {
    if source == Source::Eeprom {
        return format!("EEPROM {address:05X}");
    }
    let region = match address as u16 {
        a if a < IVT_START => "stack",
        a if a < IVT_START + IVT_SIZE => "interrupt vector table",
        _ => "program and data",
    };
    format!("{address:04X} ({region})")
}
