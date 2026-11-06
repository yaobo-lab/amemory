//! Multi-monitor workspace detection and window positioning.

use nsh_core::desktop::WindowFrame;
use nsh_core::element::desktop::set_frame;
use nsh_core::geometry::Points;

use crate::core::settings::WindowPositionMode;

/// Position the main window on screen according to the configured mode.
pub fn position_window(mode: WindowPositionMode, width: f32, height: f32) {
    let (screen_w, screen_h) = get_screen_size();

    match mode {
        WindowPositionMode::Center => {
            let x = ((screen_w - width) / 2.0).max(0.0);
            let y = ((screen_h - height) / 2.0).max(0.0);
            set_frame(WindowFrame {
                x: Some(Points(x)),
                y: Some(Points(y)),
                width: Some(Points(width)),
                height: Some(Points(height)),
                animated: Some(false),
            });
        }
        WindowPositionMode::FollowMouse => {
            let (mouse_x, mouse_y) = get_cursor_pos();
            let x = (mouse_x - width / 2.0).clamp(0.0, screen_w - width);
            let y = (mouse_y - 20.0).clamp(0.0, screen_h - height);
            set_frame(WindowFrame {
                x: Some(Points(x)),
                y: Some(Points(y)),
                width: Some(Points(width)),
                height: Some(Points(height)),
                animated: Some(false),
            });
        }
        WindowPositionMode::Remember => {
            let x = ((screen_w - width) / 2.0).max(0.0);
            let y = ((screen_h - height) / 2.0).max(0.0);
            set_frame(WindowFrame {
                x: Some(Points(x)),
                y: Some(Points(y)),
                width: Some(Points(width)),
                height: Some(Points(height)),
                animated: Some(false),
            });
        }
    }
}

fn get_screen_size() -> (f32, f32) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let w = GetSystemMetrics(SM_CXSCREEN) as f32;
        let h = GetSystemMetrics(SM_CYSCREEN) as f32;
        if w > 0.0 && h > 0.0 {
            return (w, h);
        }
    }
    (1920.0, 1080.0)
}

fn get_cursor_pos() -> (f32, f32) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            return (pt.x as f32, pt.y as f32);
        }
    }
    (960.0, 540.0)
}
