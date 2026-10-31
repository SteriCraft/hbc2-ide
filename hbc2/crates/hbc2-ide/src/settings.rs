//! What the IDE remembers between sessions.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use hbc2_asm::Target;
use hbc2_runner::Speed;
use serde::{Deserialize, Serialize};

use crate::emulator::Peripherals;

pub const STORAGE_KEY: &str = "hbc2-ide";
const MAX_RECENT: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedSetting {
    Hz(u64),
    Fastest,
}

impl From<SpeedSetting> for Speed {
    fn from(s: SpeedSetting) -> Speed {
        match s {
            SpeedSetting::Hz(hz) => Speed::Hz(hz.max(1)),
            SpeedSetting::Fastest => Speed::Fastest,
        }
    }
}

impl From<Speed> for SpeedSetting {
    fn from(s: Speed) -> SpeedSetting {
        match s {
            Speed::Hz(hz) => SpeedSetting::Hz(hz),
            Speed::Fastest => SpeedSetting::Fastest,
        }
    }
}

/// Speeds offered in the speed menu.
pub const SPEEDS: [Speed; 14] = [
    Speed::Hz(1),
    Speed::Hz(10),
    Speed::Hz(100),
    Speed::Hz(1_000),
    Speed::Hz(10_000),
    Speed::Hz(100_000),
    Speed::Hz(1_000_000),
    Speed::Hz(2_000_000),
    Speed::Hz(5_000_000),
    Speed::Hz(10_000_000),
    Speed::Hz(20_000_000),
    Speed::Hz(50_000_000),
    Speed::Hz(100_000_000),
    Speed::Fastest,
];

pub fn speed_label(speed: Speed) -> String {
    match speed {
        Speed::Fastest => "Fastest".into(),
        Speed::Hz(hz) => frequency_label(hz as f64),
    }
}

pub fn frequency_label(hz: f64) -> String {
    let (value, unit) = if hz >= 1e6 {
        (hz / 1e6, "MHz")
    } else if hz >= 1e3 {
        (hz / 1e3, "kHz")
    } else {
        (hz, "Hz")
    };
    if (value - value.round()).abs() < 0.005 {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.2} {unit}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetSetting {
    Ram,
    Eeprom,
}

impl From<TargetSetting> for Target {
    fn from(t: TargetSetting) -> Target {
        match t {
            TargetSetting::Ram => Target::Ram,
            TargetSetting::Eeprom => Target::Eeprom,
        }
    }
}

/// Remembered for each project.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub target: TargetSetting,
    /// Assembly projects: leave out the code and data nothing reaches
    /// (always on for C projects).
    pub strip: bool,
    pub peripherals: Peripherals,
    /// Breakpoint lines (1-based) of each file.
    pub breakpoints: BTreeMap<PathBuf, BTreeSet<u32>>,
    /// Files open in editor tabs.
    pub open_files: Vec<PathBuf>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        ProjectSettings {
            target: TargetSetting::Ram,
            strip: false,
            peripherals: Peripherals::default(),
            breakpoints: BTreeMap::new(),
            open_files: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub recent_projects: Vec<PathBuf>,
    /// Project open when the IDE was closed.
    pub last_project: Option<PathBuf>,
    pub speed: SpeedSetting,
    /// Show the source line of the current instruction when the emulator pauses.
    pub follow_pc: bool,
    pub editor_font_size: f32,
    /// The monitor window is open.
    pub monitor_window: bool,
    /// The CPU & I/O, Disassembly and Memory tabs are hidden.
    pub tools_hidden: bool,
    pub projects: HashMap<PathBuf, ProjectSettings>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            recent_projects: Vec::new(),
            last_project: None,
            speed: Speed::default().into(),
            follow_pc: true,
            editor_font_size: 14.0,
            monitor_window: true,
            tools_hidden: false,
            projects: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn add_recent(&mut self, root: &Path) {
        self.recent_projects.retain(|p| p != root);
        self.recent_projects.insert(0, root.to_path_buf());
        self.recent_projects.truncate(MAX_RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(speed_label(Speed::Hz(1)), "1 Hz");
        assert_eq!(speed_label(Speed::Hz(100_000)), "100 kHz");
        assert_eq!(speed_label(Speed::Hz(2_000_000)), "2 MHz");
        assert_eq!(frequency_label(1_234_567.0), "1.23 MHz");
    }

    #[test]
    fn recent_projects_are_unique_and_newest_first() {
        let mut s = Settings::default();
        s.add_recent(Path::new("a"));
        s.add_recent(Path::new("b"));
        s.add_recent(Path::new("a"));
        assert_eq!(s.recent_projects, [PathBuf::from("a"), PathBuf::from("b")]);
    }
}
