//! `hbc2-ide`: the HBC-2 IDE window.
//!
//! `hbc2-ide [project folder] [--run]` opens a project at start, and with
//! `--run` assembles and runs it.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // no console window in release

use eframe::egui;
use hbc2_ide::Ide;
use hbc2_ide::app::{Action, Saved};
use hbc2_ide::settings::STORAGE_KEY;

struct App {
    ide: Ide,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ide.ui(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.ide.saved());
    }
}

/// A crash closes the window: say why in a message box (release builds
/// have no console), and keep the details in a file.
fn report_crashes() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        let backtrace = std::backtrace::Backtrace::force_capture();
        let log = std::env::temp_dir().join("hbc2-ide-crash.log");
        let _ = std::fs::write(&log, format!("{info}\n\n{backtrace}"));
        show_error(
            "HBC-2 IDE crashed",
            &format!("{info}\n\nDetails were written to {}", log.display()),
        );
    }));
}

/// A message box above every window (an ordinary one opens behind the
/// IDE's window).
#[cfg(windows)]
fn show_error(title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
    };
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (title, text) = (wide(title), wide(text));
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND,
        );
    }
}

#[cfg(not(windows))]
fn show_error(title: &str, text: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(text)
        .show();
}

fn main() -> eframe::Result {
    report_crashes();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("HBC-2 IDE")
            .with_inner_size([1500.0, 900.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(hbc2_ide::icon()),
        ..Default::default()
    };
    eframe::run_native(
        "HBC-2 IDE",
        options,
        Box::new(|cc| {
            let saved: Option<Saved> = cc.storage.and_then(|s| eframe::get_value(s, STORAGE_KEY));
            let mut ide = Ide::new(&cc.egui_ctx, saved);
            if let Err(e) = hbc2_ide::project::create_projects_dir() {
                ide.state
                    .console
                    .error(format!("Cannot create the projects folder: {e}"));
            }
            let args: Vec<String> = std::env::args().skip(1).collect();
            if let Some(dir) = args.iter().find(|a| !a.starts_with("--")) {
                ide.state.actions.push(Action::OpenProject(dir.into()));
                if args.iter().any(|a| a == "--run") {
                    ide.state.actions.push(Action::Run);
                }
            }
            Ok(Box::new(App { ide }))
        }),
    )
}
