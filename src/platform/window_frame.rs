//! Window frame geometry, always-on-top, and frameless window controls.

use nsh_core::desktop::{TitleBarStyle, WindowLevel};
use nsh_core::element::desktop::{set_level, set_title_bar};
use nsh_core::geometry::Points;

/// Minimum window width (324px matching Clippi).
pub const MIN_WINDOW_WIDTH: f32 = 324.0;
/// Minimum window height (480px matching Clippi).
pub const MIN_WINDOW_HEIGHT: f32 = 480.0;
/// Maximum window width.
pub const MAX_WINDOW_WIDTH: f32 = 1164.0;
/// Maximum window height.
pub const MAX_WINDOW_HEIGHT: f32 = 1200.0;

/// Default window dimensions (800x835).
pub const DEFAULT_WINDOW_WIDTH: f32 = 680.0;
/// Default window height (800x835).
pub const DEFAULT_WINDOW_HEIGHT: f32 = 635.0;
/// Default window width alias.
pub const DEFAULT_WINDOW_W: f32 = DEFAULT_WINDOW_WIDTH;
/// Default window height alias.
pub const DEFAULT_WINDOW_H: f32 = DEFAULT_WINDOW_HEIGHT;

/// Setup custom title bar style with frameless drag support.
pub fn configure_title_bar(bar_height_px: f32) {
    set_title_bar(TitleBarStyle::Hidden, Some(Points(bar_height_px)));
}

/// Helper to obtain the native HWND for the application's main window on Windows.
#[cfg(target_os = "windows")]
#[must_use]
pub fn get_app_hwnd() -> isize {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowW, GetWindowThreadProcessId, IsWindowVisible,
    };

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let cur_pid = std::process::id();
        let mut w_pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &raw mut w_pid);
            if w_pid == cur_pid && IsWindowVisible(hwnd) != 0 {
                let out = lparam as *mut HWND;
                *out = hwnd;
                0 // Stop enumerating
            } else {
                1 // Continue
            }
        }
    }

    let mut found_hwnd: HWND = 0;
    unsafe {
        EnumWindows(Some(enum_proc), &raw mut found_hwnd as LPARAM);
    }
    if found_hwnd != 0 {
        return found_hwnd;
    }

    // Fallback: search by common window titles
    for title_str in &[
        crate::core::i18n_keys::I18nKey::AppName.text(),
        "Amemory 知识与记忆",
        "Amemory Knowledge & Memory",
        "amemory",
    ] {
        let title_w: Vec<u16> = title_str.encode_utf16().chain(Some(0)).collect();
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title_w.as_ptr()) };
        if hwnd != 0 {
            return hwnd;
        }
    }

    0
}

/// Set window always-on-top state.
pub fn set_always_on_top(pinned: bool) {
    if pinned {
        set_level(WindowLevel::Floating);
    } else {
        set_level(WindowLevel::Normal);
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        let hwnd = get_app_hwnd();
        if hwnd != 0 {
            let insert_after = if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
            unsafe {
                SetWindowPos(hwnd, insert_after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
        }
    }
}

/// Hide or minimize the application window to background.
pub fn hide_app_window() {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_MINIMIZE, ShowWindow};
        let hwnd = get_app_hwnd();
        if hwnd != 0 {
            unsafe {
                ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
    }
}
