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

/// Default window dimensions (525x525).
pub const DEFAULT_WINDOW_WIDTH: f32 = 525.0;
/// Default window height (525x525).
pub const DEFAULT_WINDOW_HEIGHT: f32 = 525.0;
/// Default window width alias.
pub const DEFAULT_WINDOW_W: f32 = DEFAULT_WINDOW_WIDTH;
/// Default window height alias.
pub const DEFAULT_WINDOW_H: f32 = DEFAULT_WINDOW_HEIGHT;

/// Setup custom title bar style with frameless drag support.
pub fn configure_title_bar(bar_height_px: f32) {
    set_title_bar(TitleBarStyle::Hidden, Some(Points(bar_height_px)));
}

/// Set window always-on-top state.
pub fn set_always_on_top(pinned: bool) {
    if pinned {
        set_level(WindowLevel::Floating);
    } else {
        set_level(WindowLevel::Normal);
    }
}
