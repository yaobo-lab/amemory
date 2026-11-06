//! Sidebar attached secondary window helper.

use crate::platform::tag_window::calculate_sidebar_bounds;
use crate::platform::window_frame::DEFAULT_WINDOW_W;

/// Coordinates and visibility state for attached sidebar window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarWindowState {
    pub is_visible: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SidebarWindowState {
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
