//! The HBC-2 monitor, and the HBC-2 keyboard while the monitor has focus.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, ColorImage, Event, EventFilter, Key, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, vec2,
};
use hbc2_core::peripherals::Monitor;
use hbc2_core::peripherals::monitor::{CELL, HEIGHT, PALETTE, WIDTH};
use hbc2_runner::Runner;

use crate::keymap;

/// The cursor shown by SHOW_CURSOR blinks twice a second: on for this
/// long, then off as long.
const BLINK_SECONDS: f64 = 0.25;

/// Width of the stripe around the screen while it has focus (drawn
/// outside, so that it hides no HBC-2 pixel).
const FOCUS_STRIPE: f32 = 2.0;
/// Space between the screen and the stripe, so that it does not touch the
/// characters.
const FOCUS_GAP: f32 = 1.0;
/// Room kept on each side of the screen for the stripe.
const FRAME: f32 = FOCUS_GAP + FOCUS_STRIPE;
/// Room left under the screen for the keyboard hint.
const HINT_HEIGHT: f32 = 22.0;
/// Margin of the window's panel.
const PANEL_MARGIN: f32 = 8.0;
/// The "Sharp scaling" row above the screen, with the space under it.
const HEADER_HEIGHT: f32 = 21.0;

/// Inside size of a monitor window where each HBC-2 pixel is `pixel_size`
/// points wide with sharp scaling.
pub fn window_size(pixel_size: u32) -> egui::Vec2 {
    let screen = vec2(WIDTH as f32, HEIGHT as f32) * pixel_size.max(1) as f32;
    let around = vec2(
        2.0 * (PANEL_MARGIN + FRAME),
        2.0 * (PANEL_MARGIN + FRAME) + HEADER_HEIGHT + HINT_HEIGHT,
    );
    // A little more, so that rounding does not drop to the size below.
    screen + around + vec2(2.0, 2.0)
}

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

impl Default for MonitorView {
    fn default() -> Self {
        Self {
            texture: None,
            shown: None,
            held: HashMap::new(),
            caps_shift: false,
            integer_scale: true,
        }
    }
}

pub fn color(index: u8) -> Color32 {
    let rgb = PALETTE[(index & 0x0F) as usize];
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The picture, 256 × 192 (the blinking cursor is drawn over it).
pub fn render(monitor: &Monitor) -> ColorImage {
    let pixels = monitor.pixel_buffer().iter().map(|&p| color(p)).collect();
    ColorImage::new([WIDTH, HEIGHT], pixels)
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
            let (x, y, pending) = monitor.cursor();
            let x = if pending { 32 } else { x };
            ui.weak(format!("256×192, text 32×24, cursor {x}, {y}"));
        });

        // Keep the 4:3 shape.
        let size = texture.size_vec2();
        let stripes = vec2(2.0 * FRAME, 2.0 * FRAME);
        let available = ui.available_size() - stripes - vec2(0.0, HINT_HEIGHT);
        let mut scale = (available.x / WIDTH as f32)
            .min(available.y / HEIGHT as f32)
            .max(0.25);
        if self.integer_scale && scale >= 1.0 {
            scale = scale.floor();
        }
        let screen_size = size * scale;
        let (outer, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), screen_size.y + stripes.y),
            Sense::click(),
        );
        let screen = Rect::from_center_size(outer.center(), screen_size);
        let painter = ui.painter_at(outer);
        painter.image(
            texture.id(),
            screen,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        // SHOW_CURSOR: an underline on the cursor's cell, in the text colour,
        // drawn by the "firmware" over the picture (GET_PIXEL doesn't see it).
        if monitor.cursor_shown() {
            let time = ui.input(|i| i.time);
            if ((time / BLINK_SECONDS) as u64).is_multiple_of(2) {
                let (x, y, _) = monitor.cursor();
                let cell = CELL as f32 * scale;
                let min = screen.min + vec2(x as f32 * cell, y as f32 * cell + cell - scale);
                let line = Rect::from_min_size(min, vec2(cell, scale));
                painter.rect_filled(line, 0.0, color(monitor.colors().0));
            }
            let left = BLINK_SECONDS - time % BLINK_SECONDS;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(left));
        }

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
                screen.expand(FOCUS_GAP),
                0.0,
                Stroke::new(FOCUS_STRIPE, ui.visuals().selection.stroke.color),
                StrokeKind::Outside,
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
    fn renders_the_picture() {
        let machine = Machine::new(MachineConfig::default(), None).unwrap();
        let monitor = machine.peripherals.monitor.as_ref().unwrap();
        let image = render(monitor);
        assert_eq!(image.size, [WIDTH, HEIGHT]);
        assert!(
            image.pixels.iter().all(|&p| p == color(1)),
            "blue at power-on"
        );
    }
}
