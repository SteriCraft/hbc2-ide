//! Host keys to HBC-2 keyboard scan codes.
//!
//! The HBC-2 keyboard is an AZERTY keyboard. A scan code identifies a
//! physical key, and its value is the ASCII code of what the key shows on
//! AZERTY (unshifted, upper case for letters), or a control code for the
//! other keys. This is the map of the first IDE.
//!
//! egui names physical keys after their US QWERTY position, so the AZERTY
//! "A" key is `Key::Q`, and so on.
//!
//! egui reports the keypad digits, `-` and `/` as the main-row keys. A
//! main-row key that types a digit, `-` or `/` without Shift is taken as the
//! keypad key (true on AZERTY, where those keys type `&é"'(` and `)` `!`).
//!
//! A key that types é, è, à, ù or ç is sent as the E, A, U or C key
//! instead, also with Caps Lock on. Caps Lock itself is sent as a held Shift (the HBC-2
//! has no Caps Lock state); its main-row digits are not keypad keys.
//!
//! Not available through egui: Pause (0x06), Caps Lock (0x0F), keypad Enter
//! (0x12), `*` (0x15) and `.` (0x17).

use eframe::egui::Key;

/// Scan code of the Shift keys.
pub const SHIFT: u8 = 0x0E;

/// Scan code of a physical key (US QWERTY name).
pub fn scan_code(key: Key) -> Option<u8> {
    use Key::*;
    Some(match key {
        Insert => 0x01,
        Home => 0x02,
        End => 0x03,
        PageUp => 0x04,
        PageDown => 0x05,
        ArrowLeft => 0x07,
        Backspace => 0x08,
        Tab => 0x09,
        ArrowUp => 0x0A,
        ArrowRight => 0x0B,
        ArrowDown => 0x0C,
        Enter => 0x0D,
        ShiftLeft | ShiftRight => SHIFT,
        ControlLeft | ControlRight => 0x10,
        AltLeft | AltRight => 0x11,
        Plus => 0x13, // only the keypad has a "+" key
        Escape => 0x1B,
        Space => 0x20,
        Slash => 0x21,         // ! §
        Num3 => 0x22,          // " 3
        CloseBracket => 0x24,  // $ £
        Quote => 0x25,         // ù %
        Num1 => 0x26,          // & 1
        Num4 => 0x27,          // ' 4
        Num5 => 0x28,          // ( 5
        Minus => 0x29,         // ) °
        Backslash => 0x2A,     // * µ
        Equals => 0x2B,        // = +
        M => 0x2C,             // , ?
        Num6 => 0x2D,          // - 6
        Comma => 0x2E,         // ; .
        Period => 0x3A,        // : /
        IntlBackslash => 0x3C, // < >
        Num0 => 0x40,          // à 0
        Q => b'A',
        B => b'B',
        C => b'C',
        D => b'D',
        E => b'E',
        F => b'F',
        G => b'G',
        H => b'H',
        I => b'I',
        J => b'J',
        K => b'K',
        L => b'L',
        Semicolon => b'M',
        N => b'N',
        O => b'O',
        P => b'P',
        A => b'Q',
        R => b'R',
        S => b'S',
        T => b'T',
        U => b'U',
        V => b'V',
        Z => b'W',
        X => b'X',
        Y => b'Y',
        W => b'Z',
        Num9 => 0x5B,        // ç 9
        OpenBracket => 0x5E, // ^ ¨
        Num8 => 0x5F,        // _ 8
        Num7 => 0x60,        // è 7
        Num2 => 0x7E,        // é 2
        Delete => 0x7F,
        _ => return None,
    })
}

/// Scan code of the keypad key that `key` stands for, when it typed `text`
/// without Shift (see the module documentation).
pub fn keypad_scan_code(key: Key, text: &str, shift: bool) -> Option<u8> {
    if shift {
        return None;
    }
    let digit = |n: u8| (text.as_bytes() == [b'0' + n]).then_some(b'0' + n);
    use Key::*;
    match key {
        Num0 => digit(0),
        Num1 => digit(1),
        Num2 => digit(2),
        Num3 => digit(3),
        Num4 => digit(4),
        Num5 => digit(5),
        Num6 => digit(6),
        Num7 => digit(7),
        Num8 => digit(8),
        Num9 => digit(9),
        Minus => (text == "-").then_some(0x14),
        Slash => (text == "/").then_some(0x16),
        _ => None,
    }
}

/// Scan code of the plain letter key for an accented letter: a key that
/// types é, è, à, ù or ç is sent as E, A, U or C, so French typed on the
/// host still reads well without accents.
///
/// With Caps Lock on, the Windows French layout types 2, 7, 0, % and 9 on
/// those keys instead, so then the physical key is used.
pub fn accent_scan_code(key: Key, text: &str, shift: bool, caps_lock: bool) -> Option<u8> {
    match text {
        "é" | "è" | "É" | "È" => return Some(b'E'),
        "à" | "À" => return Some(b'A'),
        "ù" | "Ù" => return Some(b'U'),
        "ç" | "Ç" => return Some(b'C'),
        _ => {}
    }
    if !caps_lock || shift {
        return None;
    }
    match key {
        Key::Num2 | Key::Num7 => Some(b'E'),
        Key::Num0 => Some(b'A'),
        Key::Quote => Some(b'U'),
        Key::Num9 => Some(b'C'),
        _ => None,
    }
}

/// Whether the host's Caps Lock is on (egui does not report it).
#[cfg(windows)]
pub fn caps_lock_on() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL};
    // SAFETY: GetKeyState only reads the thread's keyboard state.
    unsafe { GetKeyState(VK_CAPITAL as i32) & 1 != 0 }
}

/// Whether the host's Caps Lock is on. Elsewhere, Caps Lock gives the
/// accented capitals, which `accent_scan_code` handles from the text.
#[cfg(not(windows))]
pub fn caps_lock_on() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents() {
        let plain = |key, text| accent_scan_code(key, text, false, false);
        assert_eq!(plain(Key::Num2, "é"), Some(b'E'));
        assert_eq!(plain(Key::Num7, "è"), Some(b'E'));
        assert_eq!(plain(Key::Num0, "à"), Some(b'A'));
        assert_eq!(plain(Key::Quote, "ù"), Some(b'U'));
        assert_eq!(plain(Key::Num9, "ç"), Some(b'C'));
        assert_eq!(plain(Key::Num2, "2"), None);
        // Caps Lock: capitals on some layouts, digits on Windows French.
        assert_eq!(accent_scan_code(Key::Num2, "É", false, true), Some(b'E'));
        assert_eq!(accent_scan_code(Key::Num2, "2", false, true), Some(b'E'));
        assert_eq!(accent_scan_code(Key::Num0, "0", false, true), Some(b'A'));
        assert_eq!(accent_scan_code(Key::Quote, "%", false, true), Some(b'U'));
        assert_eq!(accent_scan_code(Key::Num9, "9", false, true), Some(b'C'));
        assert_eq!(accent_scan_code(Key::Num9, "Ç", false, true), Some(b'C'));
        assert_eq!(accent_scan_code(Key::Num1, "1", false, true), None);
        // Shift still types the digit.
        assert_eq!(accent_scan_code(Key::Num2, "2", true, true), None);
    }

    #[test]
    fn azerty_positions() {
        assert_eq!(scan_code(Key::Q), Some(b'A'));
        assert_eq!(scan_code(Key::W), Some(b'Z'));
        assert_eq!(scan_code(Key::Semicolon), Some(b'M'));
        assert_eq!(scan_code(Key::Num1), Some(b'&'));
        assert_eq!(scan_code(Key::Enter), Some(0x0D));
        assert_eq!(scan_code(Key::F1), None);
    }

    #[test]
    fn keypad_keys() {
        // AZERTY main row: "&" is not a digit, so it stays the "&" key.
        assert_eq!(keypad_scan_code(Key::Num1, "&", false), None);
        assert_eq!(keypad_scan_code(Key::Num1, "1", false), Some(b'1'));
        // Shift + main row types the digit on AZERTY: still the main-row key.
        assert_eq!(keypad_scan_code(Key::Num1, "1", true), None);
        assert_eq!(keypad_scan_code(Key::Minus, "-", false), Some(0x14));
        assert_eq!(keypad_scan_code(Key::Minus, ")", false), None);
    }
}
