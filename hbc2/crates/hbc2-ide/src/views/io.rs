//! IOD and peripherals: buses, interrupt lines and queue, ports, and each
//! peripheral's state.

use eframe::egui::{self, RichText};
use hbc2_core::iod::RESERVED_PORT;
use hbc2_core::peripherals::monitor::Mode;
use hbc2_core::peripherals::{Eeprom, Keyboard, Monitor, Peripheral, PeripheralKind, Rtc};
use hbc2_runner::Snapshot;

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

pub fn io_ui(ui: &mut egui::Ui, s: &Snapshot) {
    let mono = |text: String| RichText::new(text).monospace();
    let iod = &s.board.iod;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
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
                    ui.label("INR")
                        .on_hover_text("CPU ready to receive the interrupt (driven by the CPU)");
                    ui.label(if s.board.inr { "high" } else { "low" });
                    ui.end_row();
                });

            ui.add_space(8.0);
            let queue: Vec<_> = iod.queue().collect();
            ui.heading(format!("Interrupt queue ({})", queue.len()));
            if queue.is_empty() {
                ui.label(RichText::new("Empty").weak());
            } else {
                egui::Grid::new("queue")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for title in ["#", "Port", "Data"] {
                            ui.label(RichText::new(title).weak());
                        }
                        ui.end_row();
                        for (n, interrupt) in queue.iter().enumerate().take(64) {
                            ui.label(n.to_string());
                            ui.label(mono(format!("{}", interrupt.port)));
                            ui.label(mono(format!("{:02X}", interrupt.data)));
                            ui.end_row();
                        }
                    });
                if queue.len() > 64 {
                    ui.label(RichText::new(format!("… and {} more", queue.len() - 64)).weak());
                }
            }

            ui.add_space(8.0);
            ui.heading("Ports");
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

            ui.add_space(8.0);
            ui.heading("Peripherals");
            egui::Grid::new("peripherals")
                .num_columns(3)
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
        });
}

fn unplugged(ui: &mut egui::Ui, name: &str) {
    ui.label(name);
    ui.label(RichText::new("not plugged in").weak());
    ui.label("");
}
