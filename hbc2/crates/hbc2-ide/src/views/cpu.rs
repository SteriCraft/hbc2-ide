//! CPU state: registers, flags, PC/SP/IR, stack and clock.

use eframe::egui::{self, Color32, RichText, Sense, Stroke, StrokeKind, vec2};
use hbc2_asm::Program;
use hbc2_asm::disasm::disassemble_with;
use hbc2_core::{CpuState, Flag, Register};
use hbc2_runner::{RunState, Snapshot};

use crate::settings::{frequency_label, speed_label};

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

pub fn cpu_ui(
    ui: &mut egui::Ui,
    s: &Snapshot,
    baseline: Option<&Snapshot>,
    program: Option<&Program>,
) {
    let dark = ui.visuals().dark_mode;
    let mono = |text: String, changed: bool| {
        let text = RichText::new(text).monospace();
        if changed {
            text.color(changed_color(dark)).strong()
        } else {
            text
        }
    };
    let cpu = &s.cpu;
    let before = baseline.map(|b| &b.cpu);

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
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

            ui.add_space(8.0);
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

            ui.add_space(8.0);
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
                    ui.label(RichText::new(source(cpu.pc).unwrap_or_default()).weak());
                    ui.end_row();

                    ui.label("SP");
                    ui.label(mono(
                        format!("{:02X}", cpu.sp),
                        before.is_some_and(|b| b.sp != cpu.sp),
                    ));
                    ui.label(RichText::new(format!("top item at {:04X}", cpu.sp)).weak());
                    ui.end_row();

                    ui.label("IR");
                    ui.label(mono(
                        format!("{:08X}", cpu.ir.0),
                        before.is_some_and(|b| b.ir != cpu.ir),
                    ));
                    ui.label(
                        RichText::new(disassemble_with(cpu.ir.0, |a| name_of(program, a)))
                            .monospace(),
                    );
                    ui.end_row();

                    ui.label("Last PC");
                    ui.label(mono(format!("{:04X}", cpu.last_pc), false));
                    ui.label(RichText::new(source(cpu.last_pc).unwrap_or_default()).weak());
                    ui.end_row();

                    ui.label("State");
                    let state = match (cpu.state, cpu.is_halted()) {
                        (CpuState::InterruptAcknowledge, _) => "Acknowledging an interrupt",
                        (_, true) => "Halted (waiting for an interrupt)",
                        _ => "Executing",
                    };
                    ui.label(state);
                    ui.label("");
                    ui.end_row();

                    if let Some((port, data)) = cpu.pending_software_interrupt() {
                        ui.label("INT");
                        ui.label(mono(format!("port {port:02X}, data {data:02X}"), false));
                        ui.label(RichText::new("waiting for the I flag").weak());
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
            ui.heading("Stack");
            ui.label(RichText::new("Top of the stack first. SP points at the top item.").weak());
            egui::Grid::new("stack")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    for depth in 0..8u8 {
                        let address = cpu.sp.wrapping_sub(depth);
                        let value = s.board.ram.read(address as u16);
                        let changed =
                            baseline.is_some_and(|b| b.board.ram.read(address as u16) != value);
                        let marker = if depth == 0 { "SP ▶" } else { "" };
                        ui.label(RichText::new(format!("{marker} {address:04X}")).monospace());
                        ui.label(mono(format!("{value:02X}"), changed));
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
            ui.heading("Clock");
            egui::Grid::new("clock").num_columns(2).show(ui, |ui| {
                ui.label("Cycles");
                ui.label(RichText::new(group_digits(s.ticks)).monospace());
                ui.end_row();
                ui.label("Target");
                ui.label(speed_label(s.speed));
                ui.end_row();
                ui.label("Measured");
                ui.label(match s.state {
                    RunState::Running => frequency_label(s.measured_hz),
                    RunState::Paused => "paused".into(),
                });
                ui.end_row();
            });
        });
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
