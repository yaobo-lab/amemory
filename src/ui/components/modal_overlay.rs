//! Semi-transparent overlay backdrop for modal dialogs.

use nsh_core::prelude::*;

use crate::ui::theme::ClippiTheme;

/// Semi-transparent dark overlay mask.
pub fn modal_overlay(on_click_outside: impl Fn() + 'static) -> impl IntoElement {
    view()
        .absolute()
        .w_full()
        .h_full()
        .bg(rgba(0x00000066))
        .on_press(on_click_outside)
}

/// Modal overlay container wrapping dialog content in center.
pub fn modal_overlay_view(
    content: impl IntoElement + 'static,
    _theme: ClippiTheme,
    on_close: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .absolute()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .child(modal_overlay(on_close))
        .child(content)
}
