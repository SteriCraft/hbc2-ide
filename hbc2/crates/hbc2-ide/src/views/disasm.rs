//! Disassembly of the instructions in RAM, read live (self-modifying code shows).

use std::sync::Arc;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Layout, RichText, Sense, TextStyle, vec2,
};
use hbc2_asm::Program;
use hbc2_asm::disasm::disassemble_with;
use hbc2_core::Ram;
use hbc2_core::isa::Instruction;
use hbc2_runner::Snapshot;

use crate::emulator::ram_address;
use crate::views::cpu::name_of;

/// One instruction row.
#[derive(Clone, Debug)]
struct Row {
    address: u16,
    /// Label starting at this address.
    label: Option<String>,
    /// "file:line" of the source.
    source: Option<String>,
}

pub struct DisasmView {
    pub follow_pc: bool,
    /// Rows for this program (compared by pointer).
    rows_for: Option<Arc<Program>>,
    rows: Vec<Row>,
    /// PC last scrolled to.
    followed: Option<(u64, u16)>,
}

impl Default for DisasmView {
    fn default() -> Self {
        DisasmView {
            follow_pc: true,
            rows_for: None,
            rows: Vec::new(),
            followed: None,
        }
    }
}

fn program_rows(program: &Program) -> Vec<Row> {
    program
        .lines
        .iter()
        .filter_map(|line| {
            let address = ram_address(program.target, line.address)?;
            let file = program.files.get(line.file)?.file_name()?.to_string_lossy();
            Some(Row {
                address,
                label: program.symbol_at(line.address).map(|s| s.name.clone()),
                source: Some(format!("{file}:{}", line.line)),
            })
        })
        .collect()
}

/// Without debug information: the instructions from the PC on, as RAM holds
/// them now. Instructions are 1 to 4 bytes, so the ones before the PC can't
/// be found.
fn raw_rows(pc: u16, ram: &Ram) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut address = pc;
    for _ in 0..128 {
        rows.push(Row {
            address,
            label: None,
            source: None,
        });
        let Some(next) = address.checked_add(instruction_at(ram, address).size()) else {
            break; // end of RAM
        };
        address = next;
    }
    rows
}

fn instruction_at(ram: &Ram, address: u16) -> Instruction {
    Instruction::fetched(ram.read_u32(address))
}

/// The bytes of the instruction, e.g. "748005" for `mov a, 5`.
fn hex_bytes(i: Instruction) -> String {
    i.to_bytes()[..i.size() as usize]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect()
}

/// A left-aligned column of fixed width.
fn column(ui: &mut egui::Ui, width: f32, height: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        vec2(width, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(width);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

impl DisasmView {
    /// Returns an address whose breakpoint the user toggled.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        s: &Snapshot,
        program: Option<&Arc<Program>>,
        follow: bool,
    ) -> Option<u16> {
        let mut toggled = None;
        let pc = s.cpu.pc;

        match program {
            Some(p) if !self.rows_for.as_ref().is_some_and(|r| Arc::ptr_eq(r, p)) => {
                self.rows = program_rows(p);
                self.rows_for = Some(p.clone());
            }
            Some(_) => {}
            None => {
                self.rows_for = None;
                if !self.rows.iter().any(|r| r.address == pc)
                    || self.rows.first().is_some_and(|r| r.source.is_some())
                {
                    self.rows = raw_rows(pc, &s.board.ram);
                }
            }
        }
        // PC outside the program (in data, or no debug info for it): add raw rows around it.
        let pc_row = self.rows.iter().position(|r| r.address == pc);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.follow_pc, "Follow PC");
            if pc_row.is_none() {
                ui.label(
                    RichText::new(format!("PC {pc:04X} is outside the listed instructions")).weak(),
                );
            }
        });
        ui.separator();

        let row_height = ui.text_style_height(&TextStyle::Monospace) + 2.0;
        let spacing = ui.spacing().item_spacing.y;
        let mut area = egui::ScrollArea::both().auto_shrink(false);
        if self.follow_pc && follow && self.followed != Some((s.ticks, pc)) {
            self.followed = Some((s.ticks, pc));
            if let Some(index) = pc_row {
                let offset = (index as f32 - 5.0).max(0.0) * (row_height + spacing);
                area = area.vertical_scroll_offset(offset);
            }
        }

        let dark = ui.visuals().dark_mode;
        let program = program.map(|p| &**p);
        area.show_rows(ui, row_height, self.rows.len(), |ui, range| {
            for row in &self.rows[range] {
                let word = s.board.ram.read_u32(row.address);
                let instruction = instruction_at(&s.board.ram, row.address);
                let is_pc = row.address == pc;
                let has_breakpoint = s.breakpoints.contains(row.address);
                ui.horizontal(|ui| {
                    // Breakpoint dot (click to toggle) and PC arrow.
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(row_height, row_height), Sense::click());
                    if has_breakpoint {
                        ui.painter().circle_filled(
                            rect.center(),
                            5.0,
                            Color32::from_rgb(220, 50, 50),
                        );
                    } else if response.hovered() {
                        ui.painter().circle_stroke(
                            rect.center(),
                            4.5,
                            (1.0, Color32::from_rgb(220, 50, 50)),
                        );
                    }
                    if response.clicked() {
                        toggled = Some(row.address);
                    }
                    let (arrow, _) =
                        ui.allocate_exact_size(vec2(row_height, row_height), Sense::hover());
                    if is_pc {
                        ui.painter().text(
                            arrow.center(),
                            Align2::CENTER_CENTER,
                            "▶",
                            FontId::proportional(11.0),
                            Color32::from_rgb(240, 200, 40),
                        );
                    }

                    let mut address = RichText::new(format!("{:04X}", row.address)).monospace();
                    if is_pc {
                        address = address.background_color(crate::views::editor::pc_color(dark));
                    }
                    ui.label(address);
                    let bytes = format!("{:<8}", hex_bytes(instruction));
                    ui.label(RichText::new(bytes).monospace().weak());
                    let label = row
                        .label
                        .as_ref()
                        .map(|l| format!("{l}:"))
                        .unwrap_or_default();
                    let label = RichText::new(label)
                        .monospace()
                        .color(crate::highlight::color(
                            crate::highlight::Kind::LabelDef,
                            dark,
                        ));
                    column(ui, 110.0, row_height, label);
                    let text = disassemble_with(word, |a| name_of(program, a));
                    let mut text = RichText::new(text).monospace();
                    if is_pc {
                        text = text.strong();
                    }
                    column(ui, 170.0, row_height, text);
                    if let Some(source) = &row.source {
                        ui.label(RichText::new(source).weak());
                    }
                });
            }
        });
        toggled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hbc2_core::Opcode;
    use hbc2_core::isa::encode;

    #[test]
    fn raw_rows_follow_the_instruction_sizes() {
        let mut ram = Ram::new();
        let mov = encode::reg_imm8(Opcode::Mov, hbc2_core::Register::A, 5).to_bytes();
        ram.as_mut_slice()[0x0400..0x0403].copy_from_slice(&mov[..3]);
        // Then zeros: 1-byte NOPs.
        let rows = raw_rows(0x0400, &ram);
        assert_eq!(rows[0].address, 0x0400);
        assert_eq!(rows[1].address, 0x0403);
        assert_eq!(rows[2].address, 0x0404);
        assert_eq!(hex_bytes(instruction_at(&ram, 0x0400)), "748005");
        let rows = raw_rows(0xFFF0, &ram);
        assert_eq!(
            rows.last().unwrap().address,
            0xFFFF,
            "stops at the end of RAM"
        );
    }
}
