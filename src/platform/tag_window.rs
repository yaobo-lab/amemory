//! Attached left tag sidebar coordinate calculations.

/// Width of the attached tag sidebar in points.
pub const SIDEBAR_WIDTH: f32 = 56.0;
/// Vertical offset from the main window's top edge.
pub const SIDEBAR_TOP_OFFSET: f32 = 65.0;
/// Bottom padding for tag sidebar.
pub const SIDEBAR_BOTTOM_PADDING: f32 = 8.0;

/// Calculate the screen bounds of the attached sidebar given the main window's frame.
#[must_use]
pub fn calculate_sidebar_bounds(
    main_x: f32,
    main_y: f32,
    _main_width: f32,
    main_height: f32,
) -> (f32, f32, f32, f32) {
    let sidebar_x = main_x - SIDEBAR_WIDTH;
    let sidebar_y = main_y + SIDEBAR_TOP_OFFSET;
    let sidebar_w = SIDEBAR_WIDTH;
    let sidebar_h = (main_height - SIDEBAR_TOP_OFFSET - SIDEBAR_BOTTOM_PADDING).max(100.0);
    (sidebar_x, sidebar_y, sidebar_w, sidebar_h)
}
