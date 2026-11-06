//! Scaled font size calculation helpers.

use nsh_core::geometry::Points;

/// Calculate scaled font size in points from base size.
#[must_use]
pub fn fs(base_pt: f32, scale: f32) -> Points {
    Points(base_pt * scale.clamp(0.80, 1.40))
}
