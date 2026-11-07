//! Sidebar attached secondary window helper.

use crate::platform::tag_window::calculate_sidebar_bounds;
use crate::platform::window_frame::DEFAULT_WINDOW_W;

/// Coordinates and visibility state for attached sidebar window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarWindowState {
    /// Whether the sidebar window is currently visible.
    pub is_visible: bool,
    /// X screen coordinate in logical pixels.
    pub x: f32,
    /// Y screen coordinate in logical pixels.
    pub y: f32,
    /// Window width in logical pixels.
    pub width: f32,
    /// Window height in logical pixels.
    pub height: f32,
}

impl SidebarWindowState {
    /// Calculate attached sidebar position from main window coordinates.
    #[must_use]
    pub fn for_main_window(main_x: f32, main_y: f32, main_h: f32) -> Self {
        let (x, y, width, height) = calculate_sidebar_bounds(main_x, main_y, DEFAULT_WINDOW_W, main_h);
        Self {
            is_visible: false,
            x,
            y,
            width,
            height,
        }
    }
}
