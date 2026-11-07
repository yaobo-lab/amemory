//! Loading and sync spinner indicator.

use nsh_core::prelude::*;

use crate::ui::assets::glyphs;
use crate::ui::theme::ClippiTheme;

/// Render sync or loading indicator.
#[must_use]
pub fn loading_spinner(theme: ClippiTheme) -> impl IntoElement {
    view()
        .size(px(16.0))
        .items_center()
        .justify_center()
        .child(
            text(glyphs::SYNC)
                .text_color(theme.accent)
                .text_sm(),
        )
}
