//! Window frame geometry, always-on-top, and frameless window controls.

use nsh_core::desktop::{TitleBarStyle, WindowFrame, WindowLevel};
use nsh_core::element::desktop::{set_level, set_title_bar};
use nsh_core::geometry::Points;

/// Minimum and maximum dimensions matching Clippi specifications.
pub const MIN_WINDOW_WIDTH: f32 = 324.0;
pub const MIN_WINDOW_HEIGHT: f32 = 480.0;
pub const MAX_WINDOW_WIDTH: f32 = 1164.0;
pub const MAX_WINDOW_HEIGHT: f32 = 1200.0;

/// Default window dimensions.
pub const DEFAULT_WINDOW_WIDTH: f32 = 324.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 480.0;

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
