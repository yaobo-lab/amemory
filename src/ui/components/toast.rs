//! Floating Toast notification popup.

use nsh_core::prelude::*;

use crate::state::app::ToastNotification;
use crate::ui::theme::ClippiTheme;

/// Render floating bottom toast banner from `ToastNotification`.
#[must_use]
pub fn toast_banner(toast: ToastNotification, theme: ClippiTheme) -> impl IntoElement {
    toast_view(toast.message, theme)
}

/// Render floating bottom toast banner from message str.
pub fn toast_view(message: impl Into<String>, theme: ClippiTheme) -> impl IntoElement {
    let message = message.into();
    let bg = theme.toast_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;

    view()
        .absolute()
        .bottom(px(34.0))
        .self_center()
        .px(px(14.0))
        .py(px(6.0))
        .rounded(px(8.0))
        .bg(bg)
        .border(px(1.0))
        .border_color(divider)
        .items_center()
        .justify_center()
        .child(
            text(message)
                .text_color(text_1)
                .text_xs(),
        )
}

/// Render compact hover tooltip pill for action buttons.
#[must_use]
pub fn tooltip_box(label: impl Into<String>, theme: ClippiTheme) -> nsh_base::TooltipElement {
    let label = label.into();
    nsh_base::tooltip()
        .delay(180.0)
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(theme.toast_bg)
        .border(px(1.0))
        .border_color(theme.divider)
        .items_center()
        .justify_center()
        .child(
            text(label)
                .text_color(theme.text_1)
                .text_xs(),
        )
}
