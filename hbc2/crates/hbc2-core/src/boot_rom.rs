//! The boot ROM of the hardware CPU (the Logisim design in `logisim/`).
//!
//! After every reset the CPU copies a 64 KiB ROM into RAM, byte for byte
//! (`logisim/notes/ir-mem-decisions.md`): the ROM holds the program's power-on
//! RAM image, the same 64 KiB as `hbc2 asm` writes. The ROM is two AT28C256
//! (32 KiB each): the low chip holds 0000-7FFF, the high chip 8000-FFFF.
//! [`split`] gives one file per chip, address 0 of the file at the chip's
//! address 0, as an EEPROM programmer writes it.

use std::path::{Path, PathBuf};

use crate::isa::RAM_SIZE;

/// Bytes in one AT28C256.
pub const CHIP_SIZE: usize = 0x8000;

/// The low (0000-7FFF) and high (8000-FFFF) chips' contents of a 64 KiB RAM
/// image; an error if the image is not 64 KiB.
pub fn split(image: &[u8]) -> Result<[&[u8]; 2], String> {
    if image.len() != RAM_SIZE {
        return Err(format!(
            "the boot ROM holds a 64 KiB RAM image, not {} bytes",
            image.len()
        ));
    }
    let (low, high) = image.split_at(CHIP_SIZE);
    Ok([low, high])
}

/// The two files for `path`: `name.bin` gives `name_lo.bin` and `name_hi.bin`.
pub fn file_names(path: &Path) -> [PathBuf; 2] {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "boot_rom".into());
    ["lo", "hi"].map(|half| path.with_file_name(format!("{stem}_{half}.bin")))
}

/// Writes the two chips' files for `path` (see [`file_names`]) and returns
/// their paths, low first.
pub fn write(image: &[u8], path: &Path) -> Result<[PathBuf; 2], String> {
    let halves = split(image)?;
    let names = file_names(path);
    for (name, bytes) in names.iter().zip(halves) {
        std::fs::write(name, bytes).map_err(|e| format!("{}: {e}", name.display()))?;
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_follow_the_address() {
        let image: Vec<u8> = (0..RAM_SIZE).map(|a| (a >> 8) as u8 ^ a as u8).collect();
        let [low, high] = split(&image).unwrap();
        assert_eq!((low.len(), high.len()), (CHIP_SIZE, CHIP_SIZE));
        assert_eq!(low[0x1234], image[0x1234]);
        assert_eq!(high[0x1234], image[0x9234]);
        assert!(split(&image[1..]).is_err());
    }

    #[test]
    fn names() {
        let [lo, hi] = file_names(Path::new("build/prog.bin"));
        assert_eq!(lo, Path::new("build/prog_lo.bin"));
        assert_eq!(hi, Path::new("build/prog_hi.bin"));
    }
}
