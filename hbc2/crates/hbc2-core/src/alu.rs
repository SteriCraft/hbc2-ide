//! ALU arithmetic shared by the CPU and the Logisim test vectors.
//!
//! The CPU calls these functions, and `hbc2 vectors` calls them to produce
//! the expected outputs of the Logisim circuits. That way the circuits are
//! always checked against the emulator's actual behaviour.

/// ADD / ADC: `a + b + carry_in`. Returns (result, carry out).
#[inline(always)]
pub fn add(a: u8, b: u8, carry_in: bool) -> (u8, bool) {
    let sum = a as u16 + b as u16 + carry_in as u16;
    (sum as u8, sum > 0xFF)
}

/// SUB / SBB: `a - b - borrow_in`. Returns (result, borrow out). The borrow
/// is stored in the carry flag.
#[inline(always)]
pub fn sub(a: u8, b: u8, borrow_in: bool) -> (u8, bool) {
    let diff = a as i16 - b as i16 - borrow_in as i16;
    (diff as u8, diff < 0)
}
