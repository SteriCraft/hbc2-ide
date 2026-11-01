//! Keeps the monitor window above the IDE's window on Windows.
//!
//! The IDE's window becomes the monitor's owner: Windows then keeps the
//! monitor above it and brings both to the front together, while other
//! programs can still cover them. (Switching the monitor out of "always on
//! top" would not do: Windows then puts it above every ordinary window.)

/// Title of the monitor window.
pub const MONITOR_TITLE: &str = "HBC-2 Monitor";
/// End of the IDE window's title (it starts with the project's name).
const IDE_TITLE_END: &str = "HBC-2 IDE";

/// Makes the IDE's window the owner of the monitor window. False while one
/// of the two windows is not found yet.
#[cfg(windows)]
pub fn own_monitor() -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWLP_HWNDPARENT, GetWindow, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, SetWindowLongPtrW,
    };

    #[derive(Default)]
    struct Found {
        ide: Option<HWND>,
        monitor: Option<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> i32 {
        // SAFETY: `lparam` is the `Found` passed to EnumWindows below, alive
        // for the whole enumeration.
        let found = unsafe { &mut *(lparam as *mut Found) };
        let mut process = 0;
        let mut text = [0u16; 256];
        // SAFETY: `hwnd` comes from EnumWindows; both writes stay within
        // `process` and `text`.
        let len = unsafe {
            GetWindowThreadProcessId(hwnd, &mut process);
            if process != std::process::id() || IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32)
        };
        let title = String::from_utf16_lossy(&text[..len.max(0) as usize]);
        if title == MONITOR_TITLE {
            found.monitor = Some(hwnd);
        } else if title.ends_with(IDE_TITLE_END) {
            found.ide = Some(hwnd);
        }
        1
    }

    let mut found = Found::default();
    // SAFETY: `visit` only uses `found` during the call.
    unsafe { EnumWindows(Some(visit), &mut found as *mut Found as LPARAM) };
    let (Some(ide), Some(monitor)) = (found.ide, found.monitor) else {
        return false;
    };
    // SAFETY: both handles are live windows of this process.
    unsafe {
        if GetWindow(monitor, GW_OWNER) != ide {
            SetWindowLongPtrW(monitor, GWLP_HWNDPARENT, ide as isize);
        }
    }
    true
}
