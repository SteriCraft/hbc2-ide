//! Where a new window opens on the screen.

use eframe::egui::{Pos2, Rect, Vec2, ViewportInfo, vec2};

use crate::settings::Placement;

/// Height of a window's title bar (an estimate: it is not known before the
/// window opens).
const TITLE_BAR: f32 = 32.0;

/// Width of the invisible resize borders Windows puts around a window: the
/// window's position is the corner of these.
#[cfg(windows)]
const INVISIBLE_BORDER: f32 = 7.0;
#[cfg(not(windows))]
const INVISIBLE_BORDER: f32 = 0.0;

/// Position of a window whose inside is `inner_size`, at `placement` on the
/// screen of the window `main` describes. `None` while that screen is unknown.
pub fn window_position(
    main: &ViewportInfo,
    inner_size: Vec2,
    placement: Placement,
) -> Option<Pos2> {
    let area = work_area(main)?;
    Some(place(area, inner_size, placement))
}

fn place(area: Rect, inner_size: Vec2, placement: Placement) -> Pos2 {
    let size = inner_size + vec2(0.0, TITLE_BAR);
    let free = (area.size() - size).max(Vec2::ZERO);
    let (x, y) = placement.anchor();
    area.min + vec2(free.x * x, free.y * y) - vec2(INVISIBLE_BORDER, 0.0)
}

/// The part of the screen windows can use (without the taskbar), in points.
#[cfg(windows)]
fn work_area(main: &ViewportInfo) -> Option<Rect> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    let scale = main.native_pixels_per_point?;
    let center = main.outer_rect?.center() * scale;
    let point = POINT {
        x: center.x as i32,
        y: center.y as i32,
    };
    // SAFETY: MonitorFromPoint only reads the point; GetMonitorInfoW writes
    // into `info`, whose size is set as it requires.
    let work = unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        info.rcWork
    };
    let min = Pos2::new(work.left as f32, work.top as f32) / scale;
    let max = Pos2::new(work.right as f32, work.bottom as f32) / scale;
    Some(Rect::from_min_max(min, max))
}

/// The screen of the main window, assuming screens of the same size side by
/// side.
#[cfg(not(windows))]
fn work_area(main: &ViewportInfo) -> Option<Rect> {
    let size = main.monitor_size?;
    let center = main.outer_rect.map_or(Pos2::ZERO, |r| r.center());
    let origin = Pos2::new(
        (center.x / size.x).floor() * size.x,
        (center.y / size.y).floor() * size.y,
    );
    Some(Rect::from_min_size(origin, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_and_center() {
        let area = Rect::from_min_size(Pos2::new(100.0, 0.0), vec2(1000.0, 800.0));
        let size = vec2(200.0, 168.0); // 200 high with the title bar
        let at = |p| place(area, size, p) + vec2(INVISIBLE_BORDER, 0.0);
        assert_eq!(at(Placement::TopLeft), Pos2::new(100.0, 0.0));
        assert_eq!(at(Placement::Center), Pos2::new(500.0, 300.0));
        assert_eq!(at(Placement::BottomRight), Pos2::new(900.0, 600.0));
    }
}
