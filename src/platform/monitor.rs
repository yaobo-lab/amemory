//! Multi-monitor workspace detection and window positioning.

use nsh_core::desktop::WindowFrame;
use nsh_core::element::desktop::{displays, main_window};
use nsh_core::geometry::Points;

use crate::core::settings::WindowPositionMode;

/// Position the main window on screen according to the configured mode.
pub fn position_window(mode: WindowPositionMode, width: f32, height: f32) {
    match mode {
        WindowPositionMode::Center => {
            // Delegate to NSH's native DPI- and work-area-aware centering
            // (accounts for display scale factor, taskbar bounds, and window frame margins).
            main_window().center(None);
        }
        WindowPositionMode::Remember => {
            // Keep the window's current position untouched.
        }
        WindowPositionMode::FollowMouse => {
            let (win_w, win_h) = main_window().state().map_or((width, height), |st| {
                let sz = st.bounds.size;
                if sz.width.0 > 0.0 && sz.height.0 > 0.0 {
                    (sz.width.0, sz.height.0)
                } else {
                    (width, height)
                }
            });

            let (phys_x, phys_y) = get_cursor_pos();
            let all_displays = displays();
            let target_display = all_displays
                .iter()
                .find(|d| {
                    let s = d.scale_factor.max(0.5);
                    let lx = phys_x / s;
                    let ly = phys_y / s;
                    let b = d.bounds;
                    lx >= b.origin.x.0
                        && lx < b.origin.x.0 + b.size.width.0
                        && ly >= b.origin.y.0
                        && ly < b.origin.y.0 + b.size.height.0
                })
                .or_else(|| all_displays.iter().find(|d| d.primary))
                .or_else(|| all_displays.first());

            let (fx, fy) = target_display.map_or_else(
                || ((phys_x - win_w / 2.0).max(0.0), (phys_y - 20.0).max(0.0)),
                |d| {
                    let s = d.scale_factor.max(0.5);
                    let mx = phys_x / s;
                    let my = phys_y / s;
                    let area = d.visible_bounds;
                    let min_x = area.origin.x.0;
                    let min_y = area.origin.y.0;
                    let max_x = (area.origin.x.0 + area.size.width.0 - win_w).max(min_x);
                    let max_y = (area.origin.y.0 + area.size.height.0 - win_h).max(min_y);
                    (
                        (mx - win_w / 2.0).clamp(min_x, max_x),
                        (my - 20.0).clamp(min_y, max_y),
                    )
                },
            );

            main_window().set_bounds(WindowFrame {
                x: Some(Points(fx)),
                y: Some(Points(fy)),
                width: None,
                height: None,
                animated: Some(false),
            });
        }
    }
}

fn get_cursor_pos() -> (f32, f32) {
    #[cfg(windows)]
    // SAFETY: `GetCursorPos` writes to the valid stack-allocated `POINT`.
    unsafe {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&raw mut pt) != 0 {
            return (pt.x as f32, pt.y as f32);
        }
    }
    (960.0, 540.0)
}
