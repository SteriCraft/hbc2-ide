//! CPU state: registers, flags, PC/SP/IR. The stack is in the Memory tab,
//! the clock in the status bar.

use super::columns::Section;
use eframe::egui::{
    self, Align, Color32, Label, Layout, RichText, Sense, Stroke, StrokeKind, vec2,
};
use hbc2_asm::Program;
use hbc2_asm::disasm::disassemble_with;
use hbc2_core::isa::STACK_RESET;
use hbc2_core::{CpuState, Flag, Register};
use hbc2_runner::Snapshot;

pub fn changed_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(255, 200, 60)
    } else {
        Color32::from_rgb(190, 90, 0)
    }
}

/// Name of the label or variable at `address`, for disassembly.
pub fn name_of(program: Option<&Program>, address: u16) -> Option<String> {
    program?.symbol_at(address as u32).map(|s| s.name.clone())
}

/// The CPU state, as columns of sections for [`super::columns::columns`].
pub fn cpu_sections<'a>(
    dark: bool,
    s: &'a Snapshot,
    baseline: Option<&'a Snapshot>,
    program: Option<&'a Program>,
) -> Vec<Vec<Section<'a>>> {
    let mono = move |text: String, changed: bool| {
        let text = RichText::new(text).monospace();
        if changed {
            text.color(changed_color(dark)).strong()
        } else {
            text
        }
    };
    let cpu = &s.cpu;
    let before = baseline.map(|b| &b.cpu);

    vec![
        vec![Box::new(move |ui| {
            ui.heading("Registers");
            egui::Grid::new("registers")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    for title in ["", "Hex", "Dec", "Binary", "Char"] {
                        ui.label(RichText::new(title).weak());
                    }
                    ui.end_row();
                    for r in Register::ALL {
                        let value = cpu.reg(r);
                        let changed = before.is_some_and(|b| b.reg(r) != value);
                        ui.label(RichText::new(r.name().to_uppercase()).monospace().strong());
                        ui.label(mono(format!("{value:02X}"), changed));
                        ui.label(mono(format!("{value:3}"), changed));
                        ui.label(mono(
                            format!("{:04b} {:04b}", value >> 4, value & 0xF),
                            changed,
                        ));
                        let c = if (0x20..0x7F).contains(&value) {
                            (value as char).to_string()
                        } else {
                            "·".into()
                        };
                        ui.label(mono(c, changed));
                        ui.end_row();
                    }
                });
        })],
        vec![
            Box::new(move |ui| {
                ui.heading("Flags");
                ui.horizontal(|ui| {
                    // Bit 7 (H) on the left, bit 0 (C) on the right, as in the flag register.
                    for flag in Flag::ALL.iter().rev() {
                        let on = cpu.flag(*flag);
                        let changed = before.is_some_and(|b| b.flag(*flag) != on);
                        flag_box(ui, flag.letter(), on, changed, dark).on_hover_text(format!(
                            "{} (bit {}): {}",
                            flag.name(),
                            *flag as u8,
                            if on { "set" } else { "clear" }
                        ));
                    }
                    ui.label(mono(
                        format!("{:08b}", cpu.flags.0),
                        before.is_some_and(|b| b.flags != cpu.flags),
                    ));
                });
            }),
            Box::new(move |ui| {
                ui.heading("Control");
                egui::Grid::new("control")
                    .striped(true)
                    .num_columns(3)
                    .show(ui, |ui| {
                        let source = |address: u16| {
                            let program = program?;
                            let line = program.source_of(address as u32)?;
                            let file = program.files.get(line.file)?.file_name()?.to_string_lossy();
                            Some(format!("{file}:{}", line.line))
                        };
                        ui.label("PC");
                        ui.label(mono(
                            format!("{:04X}", cpu.pc),
                            before.is_some_and(|b| b.pc != cpu.pc),
                        ));
                        note(ui, RichText::new(source(cpu.pc).unwrap_or_default()).weak());
                        ui.end_row();

                        ui.label("SP");
                        ui.label(mono(
                            format!("{:04X}", cpu.sp),
                            before.is_some_and(|b| b.sp != cpu.sp),
                        ));
                        let stack = if cpu.sp == STACK_RESET {
                            "empty".to_string()
                        } else if cpu.sp < STACK_RESET {
                            let used = STACK_RESET - cpu.sp;
                            format!("{used} byte{} pushed", if used == 1 { "" } else { "s" })
                        } else {
                            "last byte pushed".to_string()
                        };
                        note(ui, RichText::new(stack).weak());
                        ui.end_row();

                        ui.label("IR");
                        ui.label(mono(
                            format!("{:08X}", cpu.ir.0),
                            before.is_some_and(|b| b.ir != cpu.ir),
                        ));
                        note(
                            ui,
                            RichText::new(disassemble_with(cpu.ir.0, |a| name_of(program, a)))
                                .monospace(),
                        );
                        ui.end_row();

                        ui.label("Last PC");
                        ui.label(mono(format!("{:04X}", cpu.last_pc), false));
                        note(
                            ui,
                            RichText::new(source(cpu.last_pc).unwrap_or_default()).weak(),
                        );
                        ui.end_row();

                        ui.label("State");
                        // Short words here, details in the note: the column keeps its width.
                        let (state, detail) = match (cpu.state, cpu.is_halted()) {
                            (CpuState::InterruptAcknowledge, _) => {
                                ("Interrupt", "being acknowledged")
                            }
                            (_, true) => ("Halted", "waiting for an interrupt"),
                            _ => ("Executing", ""),
                        };
                        ui.label(state);
                        note(ui, RichText::new(detail).weak());
                        ui.end_row();

                        if let Some((port, data)) = cpu.pending_software_interrupt() {
                            ui.label("INT");
                            ui.label(mono(format!("port {port:02X}, data {data:02X}"), false));
                            note(ui, RichText::new("waiting for the I flag").weak());
                            ui.end_row();
                        }
                    });
            }),
        ],
    ]
}

/// Width of the notes column of the control grid. Longer notes are cut
/// (hover shows them whole), so the layout does not move while running.
const NOTE_WIDTH: f32 = 190.0;

fn note(ui: &mut egui::Ui, text: RichText) {
    let height = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(
        vec2(NOTE_WIDTH, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(NOTE_WIDTH);
            ui.add(Label::new(text).truncate());
        },
    );
}

fn flag_box(
    ui: &mut egui::Ui,
    letter: char,
    on: bool,
    changed: bool,
    dark: bool,
) -> egui::Response {
    let size = vec2(22.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let visuals = ui.visuals();
    let fill = if on {
        if dark {
            Color32::from_rgb(40, 140, 70)
        } else {
            Color32::from_rgb(120, 200, 130)
        }
    } else {
        visuals.extreme_bg_color
    };
    let stroke = if changed {
        Stroke::new(2.0, changed_color(dark))
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    ui.painter()
        .rect(rect, 3.0, fill, stroke, StrokeKind::Inside);
    let text_color = if on {
        Color32::WHITE
    } else {
        visuals.weak_text_color()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        letter,
        egui::FontId::monospace(13.0),
        text_color,
    );
    response
}

pub fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn digit_groups() {
        assert_eq!(super::group_digits(0), "0");
        assert_eq!(super::group_digits(1234567), "1 234 567");
    }
}
