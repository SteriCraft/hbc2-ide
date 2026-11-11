//! The I/O: buses and interrupt lines, the mailbox card, the I/O processor
//! (its command log, and the interrupt queue and port latches of the IOD it
//! runs) and each peripheral's state.

use std::cell::Cell;

use super::columns::Section;
use eframe::egui::{self, RichText};
use hbc2_core::Iod;
use hbc2_core::iop::{Iop, LOG_SIZE, LogEntry};
use hbc2_core::mailbox::Mailbox;
use hbc2_core::peripherals::monitor::Mode;
use hbc2_core::peripherals::{Eeprom, Keyboard, Monitor, Peripheral, PeripheralKind, Rtc};
use hbc2_runner::{RunState, Snapshot};

fn owner_name(product: u16) -> String {
    match PeripheralKind::from_product(product) {
        Some(kind) => kind.name().to_string(),
        None => format!("product {product:04X}"),
    }
}

fn ports(first: Option<u8>, count: u8) -> String {
    match first {
        Some(first) => format!("ports {first}-{}", first as u16 + count as u16 - 1),
        None => "not connected".into(),
    }
}

fn mono(text: String) -> RichText {
    RichText::new(text).monospace()
}

fn hex_bytes(bytes: impl Iterator<Item = u8>) -> String {
    let text: Vec<String> = bytes.map(|b| format!("{b:02X}")).collect();
    if text.is_empty() {
        "-".into()
    } else {
        text.join(" ")
    }
}

/// The I/O, as columns of sections for [`super::columns::columns`]. With the
/// mailbox card, the first column shows the card and a second one the I/O
/// processor. `rtc_interrupt` is set when the user asks for an RTC interrupt.
pub fn io_sections<'a>(s: &'a Snapshot, rtc_interrupt: &'a Cell<bool>) -> Vec<Vec<Section<'a>>> {
    let iod = &s.board.iod;
    let mailbox = s.board.mailbox.as_ref();

    let mut first: Vec<Section<'a>> = vec![Box::new(move |ui| buses(ui, s))];
    if let Some(m) = mailbox {
        first.push(Box::new(move |ui| mailbox_card(ui, m)));
    }
    first.push(Box::new(move |ui| queue(ui, iod, mailbox.is_some())));

    let mut columns = vec![first];
    if let Some(iop) = &s.iop {
        columns.push(vec![
            Box::new(move |ui| io_processor(ui, iop)),
            Box::new(move |ui| io_log(ui, iop, iod)),
        ]);
    }
    columns.push(vec![
        Box::new(move |ui| peripherals(ui, s, rtc_interrupt)),
        Box::new(move |ui| port_latches(ui, iod, mailbox.is_some())),
    ]);
    columns
}

fn buses(ui: &mut egui::Ui, s: &Snapshot) {
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
            ui.label("INT").on_hover_text(if s.board.mailbox.is_some() {
                "Interrupt pending (driven by the mailbox card)"
            } else {
                "Interrupt pending (driven by the IOD)"
            });
            ui.label(if s.board.int { "high" } else { "low" });
            ui.end_row();
            ui.label("INR")
                .on_hover_text("CPU ready to receive the interrupt (driven by the CPU)");
            ui.label(if s.board.inr { "high" } else { "low" });
            ui.end_row();
        });
}

/// The mailbox card: its flags and latches.
fn mailbox_card(ui: &mut egui::Ui, m: &Mailbox) {
    ui.heading("Mailbox card");
    let bit = |on: bool| mono(if on { "1" } else { "0" }.into());
    egui::Grid::new("mailbox")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let status = m.status();
            let names: Vec<&str> = [(0x01, "RX_AVAIL"), (0x02, "TX_FREE"), (0x04, "EVT_PEND")]
                .into_iter()
                .filter(|(b, _)| status & b != 0)
                .map(|(_, name)| name)
                .collect();
            ui.label("STATUS")
                .on_hover_text("What in reads on port 0x01");
            ui.label(mono(format!("{status:02X}  {}", names.join(" "))));
            ui.end_row();
            ui.label("TX full")
                .on_hover_text("A byte from the CPU waits for the I/O processor");
            ui.label(bit(m.tx_full));
            ui.end_row();
            ui.label("RX full")
                .on_hover_text("A byte from the I/O processor waits for the CPU");
            ui.label(bit(m.rx_full));
            ui.end_row();
            ui.label("Event pending")
                .on_hover_text("An interrupt waits for the CPU (it drives INT)");
            ui.label(bit(m.evt_pend));
            ui.end_row();
            ui.label("VEC").on_hover_text(
                "The vector step: the card puts the port on A0-A7 and the data on the data bus",
            );
            ui.label(bit(m.vec));
            ui.end_row();
            ui.label("TX")
                .on_hover_text("The last byte the CPU wrote to port 0x00");
            ui.label(mono(format!("{:02X}", m.tx)));
            ui.end_row();
            ui.label("RX").on_hover_text("What in reads on port 0x00");
            ui.label(mono(format!("{:02X}", m.rx)));
            ui.end_row();
            ui.label("EVT")
                .on_hover_text("The last event: its port, and the data the CPU gets in I");
            ui.label(mono(format!("port {}, data {:02X}", m.port, m.data)));
            ui.end_row();
        });
}

fn queue(ui: &mut egui::Ui, iod: &Iod, mailbox: bool) {
    let queue: Vec<_> = iod.queue().collect();
    ui.heading(format!("Interrupt queue ({})", queue.len()))
        .on_hover_text(if mailbox {
            "In the I/O processor, which sends them to the CPU one at a time through EVT"
        } else {
            "In the IOD"
        });
    if queue.is_empty() {
        ui.label(RichText::new("Empty").weak());
        return;
    }
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

/// The I/O processor's side of the protocol.
fn io_processor(ui: &mut egui::Ui, iop: &Iop) {
    ui.heading("I/O processor");
    egui::Grid::new("iop")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            if iop.boot_cycles_left() > 0 {
                ui.label("Booting")
                    .on_hover_text("The firmware doesn't read the mailbox yet");
                ui.label(mono(format!("{} cycles left", iop.boot_cycles_left())));
                ui.end_row();
            }
            ui.label("Command so far")
                .on_hover_text("The bytes of a command not complete yet");
            ui.label(mono(hex_bytes(iop.partial_command().iter().copied())));
            ui.end_row();
            ui.label("Replies waiting")
                .on_hover_text("Bytes for the CPU, sent through RX one at a time");
            ui.label(mono(hex_bytes(iop.replies().copied())));
            ui.end_row();
            ui.label("Bytes from the CPU");
            ui.label(mono(iop.tx_bytes.to_string()));
            ui.end_row();
            ui.label("Bytes to the CPU");
            ui.label(mono(iop.rx_bytes.to_string()));
            ui.end_row();
            ui.label("Events sent");
            ui.label(mono(iop.events.to_string()));
            ui.end_row();
        });
}

/// What a log entry shows: what, port, value.
fn log_row(entry: LogEntry) -> (&'static str, Option<u8>, String) {
    match entry {
        LogEntry::Out { port, value } => ("OUT", Some(port), format!("{value:02X}")),
        LogEntry::In { port, value } => ("IN", Some(port), format!("-> {value:02X}")),
        LogEntry::Info => ("INFO", None, String::new()),
        LogEntry::Nop(0) => ("NOP", None, String::new()),
        LogEntry::Nop(byte) => ("ignored", None, format!("{byte:02X}")),
        LogEntry::Event { port, data } => ("event", Some(port), format!("{data:02X}")),
    }
}

/// The last commands and events, newest at the bottom.
fn io_log(ui: &mut egui::Ui, iop: &Iop, iod: &Iod) {
    ui.heading(format!("Log (last {LOG_SIZE})"))
        .on_hover_text("The commands the I/O processor ran, and the events it sent");
    if iop.log.is_empty() {
        ui.label(RichText::new("Empty").weak());
        return;
    }
    let device = |port: u8| match iod.owner(port) {
        0 => String::new(),
        owner => owner_name(owner),
    };
    egui::ScrollArea::vertical()
        .id_salt("log")
        .max_height(ui.available_height().max(60.0))
        .auto_shrink(true)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            egui::Grid::new("log")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    for title in ["#", "What", "Port", "Device", "Value"] {
                        ui.label(RichText::new(title).weak());
                    }
                    ui.end_row();
                    let first = iop.logged - iop.log.len() as u64;
                    for (n, entry) in iop.log.iter().enumerate() {
                        let (what, port, value) = log_row(*entry);
                        ui.label(mono((first + n as u64).to_string()));
                        ui.label(what);
                        ui.label(mono(port.map_or(String::new(), |p| p.to_string())));
                        ui.label(port.map_or(String::new(), device));
                        ui.label(mono(value));
                        ui.end_row();
                    }
                });
        });
}

fn peripherals(ui: &mut egui::Ui, s: &Snapshot, rtc_interrupt: &Cell<bool>) {
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
                    // Only when the RTC sends none by itself: they would
                    // pile up with its own.
                    let manual = rtc.first_port().is_some()
                        && (s.state == RunState::Paused || !s.speed.sends_rtc_interrupts());
                    let button = ui
                        .add_enabled(manual, egui::Button::new("Interrupt").small())
                        .on_hover_text(
                            "Put the date and time on the ports and send the RTC's interrupt now",
                        )
                        .on_disabled_hover_text(format!(
                            "Available while paused, or running below {} Hz (the RTC then sends no interrupts by itself)",
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
}

fn port_latches(ui: &mut egui::Ui, iod: &Iod, mailbox: bool) {
    ui.heading("Ports").on_hover_text(if mailbox {
        "The port latches in the I/O processor, reached with mbx_out / mbx_in"
    } else {
        "The IOD's port latches"
    });
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
                    if owner == 0 {
                        continue;
                    }
                    ui.label(mono(port.to_string()));
                    ui.label(owner_name(owner));
                    ui.label(mono(format!("{:02X}", iod.port(port))));
                    ui.end_row();
                }
            });
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_rows() {
        let out = log_row(LogEntry::Out {
            port: 3,
            value: 0x41,
        });
        assert_eq!(out, ("OUT", Some(3), "41".into()));
        let read = log_row(LogEntry::In {
            port: 1,
            value: 0x12,
        });
        assert_eq!(read, ("IN", Some(1), "-> 12".into()));
        assert_eq!(log_row(LogEntry::Nop(0)).0, "NOP");
        assert_eq!(log_row(LogEntry::Nop(0x7F)), ("ignored", None, "7F".into()));
        assert_eq!(hex_bytes([1u8, 0xAB].into_iter()), "01 AB");
        assert_eq!(hex_bytes(std::iter::empty()), "-");
    }
}
