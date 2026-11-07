//! Keyboard shortcut badge bubble.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Render keyboard shortcut badge.
#[must_use]
pub fn shortcut_badge(key_label: &'static str, theme: ClippiTheme) -> impl IntoElement {
    view()
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(3.0))
        .bg(theme.divider)
        .child(
            text(key_label)
                .text_color(theme.text_3)
                .text_xs(),
        )
}
