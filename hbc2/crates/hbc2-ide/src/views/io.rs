//! IOD and peripherals: buses, interrupt lines and queue, ports, and each
//! peripheral's state.

use std::cell::Cell;

use super::columns::Section;
use eframe::egui::{self, RichText};
use hbc2_core::iod::RESERVED_PORT;
use hbc2_core::peripherals::monitor::Mode;
use hbc2_core::peripherals::{Eeprom, Keyboard, Monitor, Peripheral, PeripheralKind, Rtc};
use hbc2_runner::{RunState, Snapshot};

fn owner_name(id: u8) -> String {
    match PeripheralKind::from_device_id(id) {
        Some(kind) => kind.name().to_string(),
        None => format!("device {id:02X}"),
    }
}

fn ports(first: Option<u8>, count: u8) -> String {
    match first {
        Some(first) => format!("ports {first}-{}", first as u16 + count as u16 - 1),
        None => "not connected".into(),
    }
}

/// The IOD and peripherals, as columns of sections for [`super::columns::columns`].
/// `rtc_interrupt` is set when the user asks for an RTC interrupt.
pub fn io_sections<'a>(s: &'a Snapshot, rtc_interrupt: &'a Cell<bool>) -> Vec<Vec<Section<'a>>> {
    let mono = |text: String| RichText::new(text).monospace();
    let iod = &s.board.iod;
    vec![
        vec![
            Box::new(move |ui| {
                ui.heading("Buses and lines");
                egui::Grid::new("buses")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Address bus");
                        ui.label(mono(format!("{:04X}", s.board.address_bus)));
                        ui.end_row();
                        ui.label("Data bus");
                        ui.label(mono(format!("{:02X}", s.board.data_bus)));
                        ui.end_row();
                        ui.label("INT")
                            .on_hover_text("Interrupt pending (driven by the IOD)");
                        ui.label(if s.board.int { "high" } else { "low" });
                        ui.end_row();
                        ui.label("INR").on_hover_text(
                            "CPU ready to receive the interrupt (driven by the CPU)",
                        );
                        ui.label(if s.board.inr { "high" } else { "low" });
                        ui.end_row();
                    });
            }),
            Box::new(move |ui| {
                let queue: Vec<_> = iod.queue().collect();
                ui.heading(format!("Interrupt queue ({})", queue.len()));
                if queue.is_empty() {
                    ui.label(RichText::new("Empty").weak());
                } else {
                    scroll_to_bottom_of_tab(ui, "queue", |ui| {
                        egui::Grid::new("queue")
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                for title in ["#", "Port", "Data"] {
                                    ui.label(RichText::new(title).weak());
                                }
                                ui.end_row();
                                for (n, interrupt) in queue.iter().enumerate() {
                                    ui.label(n.to_string());
                                    ui.label(mono(format!("{}", interrupt.port)));
                                    ui.label(mono(format!("{:02X}", interrupt.data)));
                                    ui.end_row();
                                }
                            });
                    });
                }
            }),
        ],
        vec![
            Box::new(move |ui| {
                ui.heading("Peripherals");
                egui::Grid::new("peripherals")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        match &s.monitor {
                            Some(m) => {
                                ui.label(Monitor::NAME);
                                ui.label(ports(m.first_port(), Monitor::PORT_COUNT));
                                ui.label(match m.mode() {
                                    Mode::Text => "text mode",
                                    Mode::Pixel => "pixel mode",
                                });
                            }
                            None => unplugged(ui, Monitor::NAME),
                        }
                        ui.end_row();
                        match &s.rtc {
                            Some(rtc) => {
                                let t = rtc.now();
                                ui.label(Rtc::NAME);
                                ui.label(ports(rtc.first_port(), Rtc::PORT_COUNT));
                                ui.label(mono(format!(
                                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                    t.year, t.month, t.day, t.hour, t.minute, t.second
                                )));
                                // Only when the RTC sends none by itself:
                                // they would pile up with its own.
                                let manual = rtc.first_port().is_some()
                                    && (s.state == RunState::Paused
                                        || !s.speed.sends_rtc_interrupts());
                                let button = ui
                                    .add_enabled(manual, egui::Button::new("Interrupt").small())
                                    .on_hover_text(
                                        "Put the date and time on the ports and send the                                          RTC's interrupt now",
                                    )
                                    .on_disabled_hover_text(format!(
                                        "Available while paused, or running below {} Hz                                          (the RTC then sends no interrupts by itself)",
                                        hbc2_runner::Speed::RTC_INTERRUPT_MIN_HZ
                                    ));
                                if button.clicked() {
                                    rtc_interrupt.set(true);
                                }
                            }
                            None => unplugged(ui, Rtc::NAME),
                        }
                        ui.end_row();
                        match &s.keyboard {
                            Some(k) => {
                                ui.label(Keyboard::NAME);
                                ui.label(ports(k.first_port(), Keyboard::PORT_COUNT));
                                ui.label("");
                            }
                            None => unplugged(ui, Keyboard::NAME),
                        }
                        ui.end_row();
                        match &s.eeprom {
                            Some(e) => {
                                ui.label(Eeprom::NAME);
                                ui.label(ports(e.first_port(), Eeprom::PORT_COUNT));
                                ui.label(format!("{} KiB", e.memory().len() / 1024));
                            }
                            None => unplugged(ui, Eeprom::NAME),
                        }
                        ui.end_row();
                    });
            }),
            Box::new(move |ui| {
                ui.heading("Ports");
                scroll_to_bottom_of_tab(ui, "ports", |ui| {
                    egui::Grid::new("ports")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            for title in ["Port", "Owner", "Value"] {
                                ui.label(RichText::new(title).weak());
                            }
                            ui.end_row();
                            for port in 0..=255u8 {
                                let owner = iod.owner(port);
                                if owner == 0 && port != RESERVED_PORT {
                                    continue;
                                }
                                ui.label(mono(port.to_string()));
                                ui.label(if port == RESERVED_PORT {
                                    "IOD".to_string()
                                } else {
                                    owner_name(owner)
                                });
                                ui.label(mono(format!("{:02X}", iod.port(port))));
                                ui.end_row();
                            }
                        });
                });
            }),
        ],
    ]
}

fn unplugged(ui: &mut egui::Ui, name: &str) {
    ui.label(name);
    ui.label(RichText::new("not plugged in").weak());
    ui.label("");
}

/// A list that scrolls by itself and fills the height left in the tab, so
/// the rest of the tab never needs scrolling.
fn scroll_to_bottom_of_tab(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(ui.available_height().max(60.0))
        .auto_shrink(true)
        .show(ui, add);
}
