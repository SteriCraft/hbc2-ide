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
pub mod emulator;
pub mod font;
pub mod highlight;
pub mod keymap;
pub mod project;
pub mod settings;
pub mod tabs;
pub mod views;

pub use app::Ide;
