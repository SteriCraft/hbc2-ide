//! Keyboard shortcuts of the IDE's commands, which Settings > Shortcuts
//! can change.
//!
//! The settings keep only the shortcuts that differ from the defaults, by
//! command name, so that a new command gets its default shortcut.

use std::collections::BTreeMap;

use eframe::egui::{self, Event, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

/// A group of commands in the settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    File,
    Editor,
    Build,
    Emulator,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::File,
        Category::Editor,
        Category::Build,
        Category::Emulator,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::File => "File",
            Category::Editor => "Editor",
            Category::Build => "Build",
            Category::Emulator => "Emulator",
        }
    }
}

/// A command a shortcut can be given to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    OpenProject,
    Save,
    SaveAll,
    /// Held, R closes the other tabs and K all of them.
    CloseTab,
    Find,
    FindInFiles,
    /// Selects the word at the cursor, then its next occurrence.
    SelectNext,
    /// Selects every occurrence of the word at the cursor.
    SelectEvery,
    DuplicateLines,
    FoldAll,
    UnfoldAll,
    Build,
    /// A second shortcut for Build.
    Build2,
    Run,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    /// A second shortcut for Step out.
    StepOut2,
    Reset,
    Restart,
}

impl Command {
    pub const ALL: [Command; 21] = [
        Command::OpenProject,
        Command::Save,
        Command::SaveAll,
        Command::CloseTab,
        Command::Find,
        Command::FindInFiles,
        Command::SelectNext,
        Command::SelectEvery,
        Command::DuplicateLines,
        Command::FoldAll,
        Command::UnfoldAll,
        Command::Build,
        Command::Build2,
        Command::Run,
        Command::Pause,
        Command::StepOver,
        Command::StepInto,
        Command::StepOut,
        Command::StepOut2,
        Command::Reset,
        Command::Restart,
    ];

    /// Its name in the settings file (never changes).
    pub fn id(self) -> &'static str {
        match self {
            Command::OpenProject => "open_project",
            Command::Save => "save",
            Command::SaveAll => "save_all",
            Command::CloseTab => "close_tab",
            Command::Find => "find",
            Command::FindInFiles => "find_in_files",
            Command::SelectNext => "select_next",
            Command::SelectEvery => "select_every",
            Command::DuplicateLines => "duplicate_lines",
            Command::FoldAll => "fold_all",
            Command::UnfoldAll => "unfold_all",
            Command::Build => "build",
            Command::Build2 => "build_2",
            Command::Run => "run",
            Command::Pause => "pause",
            Command::StepOver => "step_over",
            Command::StepInto => "step_into",
            Command::StepOut => "step_out",
            Command::StepOut2 => "step_out_2",
            Command::Reset => "reset",
            Command::Restart => "restart",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Command::OpenProject => "Open project",
            Command::Save => "Save",
            Command::SaveAll => "Save all",
            Command::CloseTab => "Close tab",
            Command::Find => "Find",
            Command::FindInFiles => "Find in files",
            Command::SelectNext => "Select the next occurrence",
            Command::SelectEvery => "Select every occurrence",
            Command::DuplicateLines => "Duplicate the lines",
            Command::FoldAll => "Collapse all functions",
            Command::UnfoldAll => "Expand all",
            Command::Build => "Build / Assemble",
            Command::Build2 => "Build / Assemble (2nd)",
            Command::Run => "Run",
            Command::Pause => "Pause",
            Command::StepOver => "Step over",
            Command::StepInto => "Step into",
            Command::StepOut => "Step out",
            Command::StepOut2 => "Step out (2nd)",
            Command::Reset => "Reset",
            Command::Restart => "Restart",
        }
    }

    pub fn category(self) -> Category {
        use Command::*;
        match self {
            OpenProject | Save | SaveAll | CloseTab => Category::File,
            Find | FindInFiles | SelectNext | SelectEvery | DuplicateLines | FoldAll
            | UnfoldAll => Category::Editor,
            Build | Build2 => Category::Build,
            Run | Pause | StepOver | StepInto | StepOut | StepOut2 | Reset | Restart => {
                Category::Emulator
            }
        }
    }

    pub fn default_shortcut(self) -> Option<KeyboardShortcut> {
        const CTRL: Modifiers = Modifiers::COMMAND;
        const NONE: Modifiers = Modifiers::NONE;
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let (modifiers, key) = match self {
            Command::OpenProject => (CTRL, Key::O),
            Command::Save => (CTRL, Key::S),
            Command::SaveAll => (ctrl_shift, Key::S),
            Command::CloseTab => (CTRL, Key::W),
            Command::Find => (CTRL, Key::F),
            Command::FindInFiles => (ctrl_shift, Key::F),
            Command::SelectNext => (CTRL, Key::L),
            Command::SelectEvery => (ctrl_shift, Key::L),
            Command::DuplicateLines => (CTRL, Key::D),
            Command::FoldAll => (CTRL, Key::K),
            Command::UnfoldAll => (CTRL, Key::J),
            Command::Build => (NONE, Key::F7),
            Command::Build2 => (CTRL, Key::B),
            Command::Run => (NONE, Key::F5),
            Command::Pause => (NONE, Key::F6),
            Command::StepOver => (NONE, Key::F10),
            Command::StepInto => (NONE, Key::F11),
            Command::StepOut => (Modifiers::SHIFT, Key::F11),
            Command::StepOut2 => (Modifiers::SHIFT, Key::F10),
            Command::Reset => (CTRL, Key::F5),
            Command::Restart => (ctrl_shift, Key::F5),
        };
        Some(KeyboardShortcut::new(modifiers, key))
    }
}

/// Shortcuts of the text box and the tabs, which cannot be changed: listed
/// in the settings page. (category, what it does, keys)
pub const FIXED: [(Category, &str, &str); 9] = [
    (Category::Editor, "Undo", "Ctrl+Z"),
    (Category::Editor, "Redo", "Ctrl+Y"),
    (Category::Editor, "Redo", "Ctrl+Shift+Z"),
    (Category::Editor, "Cut", "Ctrl+X"),
    (Category::Editor, "Copy", "Ctrl+C"),
    (Category::Editor, "Paste", "Ctrl+V"),
    (Category::Editor, "Select all", "Ctrl+A"),
    (Category::Editor, "Add a cursor", "Ctrl+Click"),
    (Category::Editor, "Back to one cursor", "Escape"),
];

/// "Ctrl+Shift+Alt+Key", as the menus show it.
pub fn format(shortcut: KeyboardShortcut) -> String {
    let m = shortcut.modifiers;
    let mut text = String::new();
    if m.ctrl || m.command {
        text.push_str("Ctrl+");
    }
    if m.shift {
        text.push_str("Shift+");
    }
    if m.alt {
        text.push_str("Alt+");
    }
    text.push_str(shortcut.logical_key.name());
    text
}

/// Reads what [`format`] writes.
pub fn parse(text: &str) -> Option<KeyboardShortcut> {
    let mut parts: Vec<&str> = text.split('+').collect();
    let key = Key::from_name(parts.pop()?)?;
    let mut modifiers = Modifiers::NONE;
    for part in parts {
        modifiers = modifiers.plus(match part {
            "Ctrl" => Modifiers::COMMAND,
            "Shift" => Modifiers::SHIFT,
            "Alt" => Modifiers::ALT,
            _ => return None,
        });
    }
    Some(KeyboardShortcut::new(modifiers, key))
}

/// Why a key cannot be a shortcut, if it cannot.
pub fn refusal(shortcut: KeyboardShortcut) -> Option<String> {
    let m = shortcut.modifiers;
    // F1 to F35.
    let function_key = shortcut
        .logical_key
        .name()
        .strip_prefix('F')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !(m.ctrl || m.command || m.alt || function_key) {
        return Some("A shortcut needs Ctrl or Alt, unless it is a function key (F1…F12).".into());
    }
    let text = format(shortcut);
    FIXED
        .iter()
        .find(|(_, _, keys)| *keys == text)
        .map(|(_, what, _)| format!("{text} is the text box's {what}: it cannot be changed."))
}

/// The shortcuts that differ from the defaults: command id → shortcut
/// ("" for none).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shortcuts {
    changed: BTreeMap<String, String>,
}

impl Shortcuts {
    pub fn get(&self, command: Command) -> Option<KeyboardShortcut> {
        match self.changed.get(command.id()) {
            Some(text) => parse(text),
            None => command.default_shortcut(),
        }
    }

    /// The shortcut as the menus show it ("" for none).
    pub fn text(&self, command: Command) -> String {
        self.get(command).map(format).unwrap_or_default()
    }

    /// "`text` (F7)", or `text` alone when the command has no shortcut.
    pub fn with_key(&self, text: &str, command: Command) -> String {
        match self.get(command) {
            Some(s) => format!("{text} ({})", format(s)),
            None => text.to_string(),
        }
    }

    /// The keys that close several tabs: `letter` pressed while Close tab
    /// is held ("Ctrl+W+R"; "" when Close tab has no shortcut).
    pub fn close_chord(&self, letter: char) -> String {
        match self.get(Command::CloseTab) {
            Some(s) => format!("{}+{letter}", format(s)),
            None => String::new(),
        }
    }

    fn set(&mut self, command: Command, shortcut: Option<KeyboardShortcut>) {
        if shortcut == command.default_shortcut() {
            self.changed.remove(command.id());
        } else {
            let text = shortcut.map(format).unwrap_or_default();
            self.changed.insert(command.id().to_string(), text);
        }
    }

    /// Gives `shortcut` to `command`. Another command that had it loses
    /// it: it is returned.
    pub fn assign(
        &mut self,
        command: Command,
        shortcut: Option<KeyboardShortcut>,
    ) -> Option<Command> {
        let shortcut = shortcut.map(normalized);
        let previous = shortcut.and_then(|s| {
            Command::ALL
                .into_iter()
                .find(|&c| c != command && self.get(c).map(normalized) == Some(s))
        });
        if let Some(other) = previous {
            self.set(other, None);
        }
        self.set(command, shortcut);
        previous
    }

    pub fn is_default(&self) -> bool {
        self.changed.is_empty()
    }

    pub fn restore_defaults(&mut self) {
        self.changed.clear();
    }
}

/// The same shortcut, with Ctrl written one way.
fn normalized(shortcut: KeyboardShortcut) -> KeyboardShortcut {
    parse(&format(shortcut)).unwrap_or(shortcut)
}

/// Takes the presses of `shortcut` (with exactly its modifiers) out of the
/// input. Returns whether there was one.
pub fn consume(input: &mut egui::InputState, shortcut: KeyboardShortcut) -> bool {
    let before = input.events.len();
    input.events.retain(|event| {
        !matches!(event, Event::Key { key, pressed: true, modifiers, .. }
            if *key == shortcut.logical_key && modifiers.matches_exact(shortcut.modifiers))
    });
    input.events.len() != before
}

/// Takes the presses of a command's shortcut out of the input.
pub fn consume_command(ctx: &egui::Context, shortcuts: &Shortcuts, command: Command) -> bool {
    shortcuts
        .get(command)
        .is_some_and(|s| ctx.input_mut(|i| consume(i, s)))
}

/// Shift, Ctrl, Alt and the Windows key: egui sends them as keys too.
pub fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        for command in Command::ALL {
            let shortcut = command.default_shortcut().unwrap();
            assert_eq!(parse(&format(shortcut)), Some(shortcut), "{command:?}");
        }
        assert_eq!(
            format(Command::SaveAll.default_shortcut().unwrap()),
            "Ctrl+Shift+S"
        );
        assert_eq!(
            parse("Ctrl+Minus").map(format).as_deref(),
            Some("Ctrl+Minus")
        );
        assert_eq!(parse("Hyper+K"), None);
    }

    #[test]
    fn defaults_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for command in Command::ALL {
            assert!(seen.insert(command.default_shortcut()), "{command:?}");
            assert!(refusal(command.default_shortcut().unwrap()).is_none());
        }
    }

    #[test]
    fn assigning_takes_from_the_other_command() {
        let mut s = Shortcuts::default();
        let ctrl_k = parse("Ctrl+K");
        assert_eq!(s.assign(Command::Save, ctrl_k), Some(Command::FoldAll));
        assert_eq!(s.get(Command::Save), ctrl_k);
        assert_eq!(s.get(Command::FoldAll), None);
        assert_eq!(s.text(Command::FoldAll), "");
        // Back to its default: only the other change is kept.
        assert_eq!(s.assign(Command::Save, parse("Ctrl+S")), None);
        assert!(!s.is_default());
        s.restore_defaults();
        assert_eq!(s.get(Command::FoldAll), ctrl_k);
    }

    #[test]
    fn refusals() {
        assert!(refusal(parse("K").unwrap()).is_some());
        assert!(refusal(parse("Ctrl+C").unwrap()).is_some());
        assert!(refusal(parse("F2").unwrap()).is_none());
        assert!(refusal(parse("Alt+K").unwrap()).is_none());
        assert!(refusal(parse("Escape").unwrap()).is_some());
    }
}
