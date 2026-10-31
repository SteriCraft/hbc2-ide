//! HBC-2 IDE: edit, assemble and debug HBC-2 programs, and watch the CPU and
//! peripherals while the emulator runs.
//!
//! The emulator runs on its own thread (`hbc2-runner`); the GUI reads the
//! latest state snapshot once per frame and sends commands back. See
//! [`app`] for how the IDE's state and actions are organised.

pub mod app;
mod chrome;
pub mod console;
pub mod document;
pub mod editing;
pub mod emulator;
pub mod find;
pub mod font;
pub mod highlight;
pub mod keymap;
pub mod project;
pub mod settings;
pub mod signatures;
pub mod tabs;
pub mod views;

pub use app::Ide;

/// The HBC-2 icon, for the windows.
pub fn icon() -> eframe::egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("the icon is a valid PNG")
}
