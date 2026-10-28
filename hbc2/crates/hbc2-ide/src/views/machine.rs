//! The "CPU & I/O" tab: the CPU sections, then the I/O sections below them.
//! Everything shows at once; only the interrupt queue and the ports scroll.

use std::cell::Cell;

use eframe::egui::{self, RichText};
use hbc2_asm::Program;
use hbc2_runner::Snapshot;

use super::{columns, cpu, io};

/// Shows the tab. Returns the width it needs to show all its text, and
/// whether the user asked for an RTC interrupt.
pub fn machine_ui(
    ui: &mut egui::Ui,
    s: &Snapshot,
    baseline: Option<&Snapshot>,
    program: Option<&Program>,
) -> (f32, bool) {
    let dark = ui.visuals().dark_mode;
    let margin = ui.spacing().item_spacing.x * 2.0 + 16.0;
    ui.add_space(6.0);
    area_title(ui, "CPU");
    let cpu_width = columns::columns(ui, "cpu", cpu::cpu_sections(dark, s, baseline, program));
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(2.0);
    area_title(ui, "I/O");
    let rtc_interrupt = Cell::new(false);
    let io_width = columns::columns(ui, "io", io::io_sections(s, &rtc_interrupt));
    (cpu_width.max(io_width) + margin, rtc_interrupt.get())
}

fn area_title(ui: &mut egui::Ui, title: &str) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).size(24.0).strong());
    });
    ui.add_space(4.0);
}
