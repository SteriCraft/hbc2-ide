//! The HBC-2 monitor, and the HBC-2 keyboard while the monitor has focus.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, ColorImage, Event, EventFilter, Key, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, vec2,
};
use hbc2_core::peripherals::Monitor;
use hbc2_core::peripherals::monitor::{
    CHAR_HEIGHT, CHAR_WIDTH, HEIGHT, Mode, PALETTE, TEXT_COLUMNS, TEXT_ROWS, WIDTH,
};
use hbc2_runner::Runner;

use crate::{font, keymap};

/// Text mode is 252 pixels wide, pixel mode 256.
const TEXT_WIDTH: usize = TEXT_COLUMNS * CHAR_WIDTH;

#[derive(Default)]
pub struct MonitorView {
    texture: Option<TextureHandle>,
    /// The monitor state the texture shows.
    shown: Option<Arc<Monitor>>,
    /// Keys held down on the HBC-2 keyboard: host key → scan code sent.
    held: HashMap<Key, u8>,
    /// Shift is held on the HBC-2 keyboard because the host's Caps Lock is on.
    caps_shift: bool,
    pub integer_scale: bool,
}

pub fn color(index: u8) -> Color32 {
    let rgb = PALETTE[(index & 0x0F) as usize];
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Renders the monitor into an image (252×192 in text mode, 256×192 in pixel mode).
pub fn render(monitor: &Monitor) -> ColorImage {
    match monitor.mode() {
        Mode::Pixel => {
            let pixels = monitor.pixel_buffer().iter().map(|&p| color(p)).collect();
            ColorImage::new([WIDTH, HEIGHT], pixels)
        }
        Mode::Text => {
            let mut pixels = vec![Color32::BLACK; TEXT_WIDTH * HEIGHT];
            for (index, cell) in monitor.text_buffer().iter().enumerate() {
                let (column, row) = (index % TEXT_COLUMNS, index / TEXT_COLUMNS);
                let (fg, bg) = (color(cell.foreground()), color(cell.background()));
                let glyph = font::glyph(cell.ascii);
                for y in 0..CHAR_HEIGHT {
                    let bits = glyph.map_or(0, |g| g[y]);
                    let start = (row * CHAR_HEIGHT + y) * TEXT_WIDTH + column * CHAR_WIDTH;
                    for x in 0..CHAR_WIDTH {
                        let on = bits & (1 << (CHAR_WIDTH - 1 - x)) != 0;
                        pixels[start + x] = if on { fg } else { bg };
                    }
                }
            }
            debug_assert_eq!(TEXT_ROWS * CHAR_HEIGHT, HEIGHT);
            ColorImage::new([TEXT_WIDTH, HEIGHT], pixels)
        }
    }
}

impl MonitorView {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        monitor: Option<&Arc<Monitor>>,
        keyboard_plugged: bool,
        runner: &Runner,
    ) {
        let Some(monitor) = monitor else {
            ui.centered_and_justified(|ui| {
                ui.label("No monitor plugged in (Emulator > Peripherals).");
            });
            self.release_all(runner);
            return;
        };

        // Upload a new image only when the monitor changed.
        if !self.shown.as_ref().is_some_and(|s| Arc::ptr_eq(s, monitor)) {
            let image = render(monitor);
            match &mut self.texture {
                Some(texture) => texture.set(image, TextureOptions::NEAREST),
                None => {
                    self.texture = Some(ui.ctx().load_texture(
                        "hbc2-monitor",
                        image,
                        TextureOptions::NEAREST,
                    ));
                }
            }
            self.shown = Some(monitor.clone());
        }
        let Some(texture) = &self.texture else { return };

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.integer_scale, "Sharp scaling")
                .on_hover_text(
                    "Scale by whole numbers only, so every HBC-2 pixel has the same size.",
                );
            let mode = match monitor.mode() {
                Mode::Text => "text mode, 42×24",
                Mode::Pixel => "pixel mode, 256×192",
            };
            ui.weak(mode);
        });

        // Keep the 4:3 shape (text mode keeps the same height, 4 pixels narrower).
        let size = texture.size_vec2();
        let available = ui.available_size() - vec2(0.0, 22.0);
        let mut scale = (available.x / WIDTH as f32)
            .min(available.y / HEIGHT as f32)
            .max(0.25);
        if self.integer_scale && scale >= 1.0 {
            scale = scale.floor();
        }
        let screen_size = vec2(WIDTH as f32, HEIGHT as f32) * scale;
        let (outer, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), screen_size.y), Sense::click());
        let screen = Rect::from_center_size(outer.center(), screen_size);
        let image_rect = Rect::from_center_size(screen.center(), size * scale);
        let painter = ui.painter_at(outer);
        painter.rect_filled(screen, 0.0, Color32::BLACK);
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        // Keyboard: the monitor takes keys while it has focus.
        if response.clicked() {
            response.request_focus();
        }
        let focused = response.has_focus();
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
            if keyboard_plugged {
                self.forward_keys(ui, runner);
            }
            painter.rect_stroke(
                screen,
                0.0,
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                StrokeKind::Inside,
            );
        } else {
            self.release_all(runner);
        }

        let hint = match (keyboard_plugged, focused) {
            (false, _) => "No keyboard plugged in.".to_string(),
            (true, true) => {
                "Typing goes to the HBC-2 keyboard. Click elsewhere to stop.".to_string()
            }
            (true, false) => "Click the screen to type on the HBC-2 keyboard.".to_string(),
        };
        ui.vertical_centered(|ui| ui.label(RichText::new(hint).weak()));
    }

    fn forward_keys(&mut self, ui: &egui::Ui, runner: &Runner) {
        // Caps Lock holds Shift on the HBC-2 keyboard, the way capitals are typed there.
        let caps_lock = keymap::caps_lock_on();
        if caps_lock != self.caps_shift {
            self.caps_shift = caps_lock;
            if !self.held.values().any(|&scan| scan == keymap::SHIFT) {
                runner.key(keymap::SHIFT, caps_lock);
            }
        }

        let events = ui.input(|i| i.events.clone());
        for (index, event) in events.iter().enumerate() {
            let Event::Key {
                key,
                physical_key,
                pressed,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            let key = physical_key.unwrap_or(*key);
            if *pressed {
                // The text this key typed, if any, comes right after it.
                let text = events[index + 1..]
                    .iter()
                    .take_while(|e| !matches!(e, Event::Key { .. }))
                    .find_map(|e| match e {
                        Event::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                // Caps Lock makes the AZERTY main row type digits, like Shift:
                // those digits are not keypad keys.
                let scan = keymap::keypad_scan_code(key, text, modifiers.shift || caps_lock)
                    .or_else(|| keymap::accent_scan_code(key, text, modifiers.shift, caps_lock))
                    .or_else(|| self.held.get(&key).copied())
                    .or_else(|| keymap::scan_code(key));
                if let Some(scan) = scan {
                    self.held.insert(key, scan);
                    runner.key(scan, true); // repeats are sent too, like a real keyboard
                }
            } else if let Some(scan) = self.held.remove(&key).or_else(|| keymap::scan_code(key)) {
                let shift_still_held = scan == keymap::SHIFT
                    && (self.caps_shift || self.held.values().any(|&s| s == keymap::SHIFT));
                if !shift_still_held {
                    runner.key(scan, false);
                }
            }
        }
    }

    /// Releases every key still held (focus lost, monitor unplugged).
    fn release_all(&mut self, runner: &Runner) {
        let mut shift_released = false;
        for (_, scan) in self.held.drain() {
            shift_released |= scan == keymap::SHIFT;
            runner.key(scan, false);
        }
        if self.caps_shift && !shift_released {
            runner.key(keymap::SHIFT, false);
        }
        self.caps_shift = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbc2_core::{Machine, MachineConfig};

    #[test]
    fn renders_both_modes() {
        let machine = Machine::new(MachineConfig::default(), None).unwrap();
        let monitor = machine.peripherals.monitor.as_ref().unwrap();
        let image = render(monitor);
        assert_eq!(image.size, [TEXT_WIDTH, HEIGHT]);
        assert!(image.pixels.iter().all(|&p| p == Color32::BLACK));
    }
}
