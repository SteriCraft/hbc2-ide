//! What the IDE remembers between sessions.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use hbc2_asm::Target;
use hbc2_runner::Speed;
use serde::{Deserialize, Serialize};

use crate::editor_groups::EditorLayout;
use crate::emulator::Peripherals;
use crate::shortcuts::Shortcuts;

pub const STORAGE_KEY: &str = "hbc2-ide";

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
    /// How the editor tabs were split into groups.
    pub editor_layout: Option<EditorLayout>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        ProjectSettings {
            target: TargetSetting::Ram,
            strip: false,
            peripherals: Peripherals::default(),
            breakpoints: BTreeMap::new(),
            open_files: Vec::new(),
            editor_layout: None,
        }
    }
}

/// Where the monitor window opens on the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    TopLeft,
    Top,
    #[default]
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Placement {
    /// Row by row, from the top left.
    pub const ALL: [Placement; 9] = [
        Placement::TopLeft,
        Placement::Top,
        Placement::TopRight,
        Placement::Left,
        Placement::Center,
        Placement::Right,
        Placement::BottomLeft,
        Placement::Bottom,
        Placement::BottomRight,
    ];

    /// Where the window goes in the free space, from 0 (left, top) to 1
    /// (right, bottom).
    pub fn anchor(self) -> (f32, f32) {
        let index = Placement::ALL.iter().position(|&p| p == self).unwrap_or(4);
        ((index % 3) as f32 / 2.0, (index / 3) as f32 / 2.0)
    }

    pub fn label(self) -> &'static str {
        match self {
            Placement::TopLeft => "Top left",
            Placement::Top => "Top",
            Placement::TopRight => "Top right",
            Placement::Left => "Left",
            Placement::Center => "Center",
            Placement::Right => "Right",
            Placement::BottomLeft => "Bottom left",
            Placement::Bottom => "Bottom",
            Placement::BottomRight => "Bottom right",
        }
    }
}

/// Settings dialog, Monitor page.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MonitorSettings {
    /// The monitor window opens when the IDE starts.
    pub open_on_startup: bool,
    /// "Sharp scaling" is ticked when the IDE starts.
    pub sharp_scaling: bool,
    /// With sharp scaling, the size of an HBC-2 pixel when the window opens.
    pub pixel_size: u32,
    pub placement: Placement,
    /// Open where it was last closed (`placement` and `pixel_size` are used
    /// the first time only).
    pub remember_position: bool,
}

impl Default for MonitorSettings {
    fn default() -> Self {
        MonitorSettings {
            open_on_startup: true,
            sharp_scaling: true,
            pixel_size: 2,
            placement: Placement::default(),
            remember_position: false,
        }
    }
}

/// Settings dialog, General page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// The project open when the IDE closed opens again when it starts.
    pub reopen_last_project: bool,
    /// Where new projects go (`None`: Documents/hbc2_projects).
    pub projects_folder: Option<PathBuf>,
    /// How many projects File > Recent projects lists.
    pub recent_count: usize,
    /// "New project" makes a folder named after the project, by default.
    pub project_folder: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        GeneralSettings {
            reopen_last_project: true,
            projects_folder: None,
            recent_count: 10,
            project_folder: true,
        }
    }
}

/// Settings dialog, Editor page (the font size is [`Settings::editor_font_size`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    /// Tab and Shift+Tab put and take spaces, not tab characters.
    pub indent_with_spaces: bool,
    /// Columns between tab stops: how wide a tab is, and how many spaces
    /// make one indentation level.
    pub tab_width: usize,
    pub line_numbers: bool,
    /// The line of the cursor gets a lighter background.
    pub highlight_line: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            indent_with_spaces: false,
            tab_width: 4,
            line_numbers: true,
            highlight_line: true,
        }
    }
}

/// Settings dialog, Build page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildSettings {
    /// Assembly projects save their files before assembling (C projects
    /// always do).
    pub save_before_build: bool,
    /// Build (F7) runs the program when it succeeds.
    pub run_after_build: bool,
}

/// Settings dialog, Emulator page (Follow PC is [`Settings::follow_pc`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EmulatorSettings {
    /// Clock when the IDE starts (`None`: as it was left).
    pub startup_speed: Option<SpeedSetting>,
    /// Peripherals plugged in a new project.
    pub new_project_peripherals: Peripherals,
    /// A trace stops by itself after this many lines.
    pub trace_limit: u64,
}

impl EmulatorSettings {
    pub const DEFAULT_TRACE_LIMIT: u64 = 1_000_000;
}

impl Default for EmulatorSettings {
    fn default() -> Self {
        EmulatorSettings {
            startup_speed: None,
            new_project_peripherals: Peripherals::default(),
            trace_limit: EmulatorSettings::DEFAULT_TRACE_LIMIT,
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
    pub general: GeneralSettings,
    pub editor: EditorSettings,
    pub build: BuildSettings,
    pub emulator: EmulatorSettings,
    pub monitor: MonitorSettings,
    /// Settings dialog, Shortcuts page.
    pub shortcuts: Shortcuts,
    /// Where the monitor window was last closed: outside position and
    /// inside size.
    pub monitor_geometry: Option<[f32; 4]>,
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
            general: GeneralSettings::default(),
            editor: EditorSettings::default(),
            build: BuildSettings::default(),
            emulator: EmulatorSettings::default(),
            monitor: MonitorSettings::default(),
            shortcuts: Shortcuts::default(),
            monitor_geometry: None,
            projects: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn add_recent(&mut self, root: &Path) {
        self.recent_projects.retain(|p| p != root);
        self.recent_projects.insert(0, root.to_path_buf());
        self.recent_projects
            .truncate(self.general.recent_count.max(1));
    }

    /// Takes the settings of the settings dialog from `from`.
    pub fn set_preferences(&mut self, from: &Settings) {
        self.general = from.general.clone();
        self.editor = from.editor;
        self.editor_font_size = from.editor_font_size;
        self.build = from.build;
        self.emulator = from.emulator;
        self.follow_pc = from.follow_pc;
        self.monitor = from.monitor;
        self.shortcuts = from.shortcuts.clone();
        self.recent_projects
            .truncate(self.general.recent_count.max(1));
    }

    /// The folder where new projects go.
    pub fn projects_folder(&self) -> PathBuf {
        match &self.general.projects_folder {
            Some(dir) if dir.is_dir() => dir.clone(),
            _ => crate::project::default_projects_parent(),
        }
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
    fn placement_anchors() {
        assert_eq!(Placement::TopLeft.anchor(), (0.0, 0.0));
        assert_eq!(Placement::Center.anchor(), (0.5, 0.5));
        assert_eq!(Placement::Right.anchor(), (1.0, 0.5));
        assert_eq!(Placement::BottomRight.anchor(), (1.0, 1.0));
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
