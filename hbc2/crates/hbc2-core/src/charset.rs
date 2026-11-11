//! The HBC-2's character set, Windows-1252 (Peripherals/Native/Reference.md,
//! "Characters"): ASCII, the Windows-1252 additions in 0x80-0x9F, Latin-1
//! in 0xA0-0xFF. Sources are UTF-8: the assembler and the C compiler turn
//! the characters of their literals into these bytes.

/// The characters of 0x80-0x9F (`None`: undefined in Windows-1252).
const HIGH: [Option<char>; 32] = [
    Some('€'),
    None,
    Some('‚'),
    Some('ƒ'),
    Some('„'),
    Some('…'),
    Some('†'),
    Some('‡'),
    Some('ˆ'),
    Some('‰'),
    Some('Š'),
    Some('‹'),
    Some('Œ'),
    None,
    Some('Ž'),
    None,
    None,
    Some('‘'),
    Some('’'),
    Some('“'),
    Some('”'),
    Some('•'),
    Some('–'),
    Some('—'),
    Some('˜'),
    Some('™'),
    Some('š'),
    Some('›'),
    Some('œ'),
    None,
    Some('ž'),
    Some('Ÿ'),
];

/// The character of a byte (`None` for the five undefined codes).
/// Control codes map to themselves (U+0000-U+001F, U+007F).
pub fn to_char(byte: u8) -> Option<char> {
    match byte {
        0x80..=0x9F => HIGH[byte as usize - 0x80],
        _ => Some(byte as char),
    }
}

/// The byte of a character, `None` when Windows-1252 has none.
pub fn from_char(c: char) -> Option<u8> {
    match c as u32 {
        0x00..=0x7F | 0xA0..=0xFF => Some(c as u8),
        _ => HIGH
            .iter()
            .position(|&h| h == Some(c))
            .map(|i| 0x80 + i as u8),
    }
}

/// A UTF-8 text in Windows-1252, or the first character that has no byte.
pub fn encode(text: &str) -> Result<Vec<u8>, char> {
    text.chars().map(|c| from_char(c).ok_or(c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for byte in 0..=255u8 {
            if let Some(c) = to_char(byte) {
                assert_eq!(from_char(c), Some(byte), "{byte:02X}");
            }
        }
        assert_eq!(
            encode("Noël œuf €"),
            Ok(vec![
                b'N', b'o', 0xEB, b'l', b' ', 0x9C, b'u', b'f', b' ', 0x80
            ])
        );
        assert_eq!(encode("a→b"), Err('→'));
        assert_eq!(
            from_char('\u{81}'),
            None,
            "C1 control, undefined in Windows-1252"
        );
        assert_eq!((0x80..=0x9F).filter(|&b| to_char(b).is_none()).count(), 5);
    }
}
