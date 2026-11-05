//! Microcode of the HBC-2's hardware control unit (logisim/notes/cu-decisions.md).
//!
//! - [`source`]: the microcode source language (`microcode/hbc2.uc`);
//! - [`generate`]: the generator: rules, placement, the 7 AT28C256 images, the
//!   listing and the cycle-count table;
//! - [`model`]: a fast step-level model of the datapath that runs the images,
//!   checked against the emulator by this crate's tests.
//! - [`check`]: the comparisons with the emulator (tests and `hbc2 ucode --check`).

pub mod check;
pub mod generate;
pub mod model;
pub mod source;
pub mod word;

/// The microcode source, as shipped.
pub const SOURCE: &str = include_str!("../microcode/hbc2.uc");

/// Builds the shipped microcode.
pub fn build() -> Result<generate::Microcode, Vec<String>> {
    generate::build(SOURCE)
}
