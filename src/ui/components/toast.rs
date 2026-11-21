//! Floating Toast notification popup.

use nsh_core::prelude::*;

use crate::state::app::ToastNotification;
use crate::ui::theme::ClippiTheme;

/// Render floating centered toast banner from `ToastNotification`.
#[must_use]
pub fn toast_banner(toast: ToastNotification, theme: ClippiTheme) -> impl IntoElement {
    toast_view(toast.message, theme)
}

/// Render floating centered toast banner from message str.
pub fn toast_view(message: impl Into<String>, theme: ClippiTheme) -> impl IntoElement {
    let message = message.into();
    let is_error = is_error_toast(&message);
    let (bg, border_color, dot_color) = if is_error {
        (rgba(0xd64b45f2), rgba(0xffffff40), rgba(0xffd6d4ff))
    } else {
        (theme.toast_bg, theme.toast_border, rgba(0xbef5d9ff))
    };
    let text_color = theme.toast_text;

    view()
        .absolute()
        .w_full()
        .h_full()
        .pointer_events(nsh_core::PointerEvents::None)
        .items_center()
        .justify_center()
        .child(
            view()
                .max_w(px(460.0))
                .px(px(20.0))
                .py(px(11.0))
                .rounded(px(10.0))
                .bg(bg)
                .border(px(1.5))
                .border_color(border_color)
                .shadow_xl()
                .pointer_events(nsh_core::PointerEvents::None)
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(view().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(dot_color))
                .child(
                    text(message)
                        .text_color(text_color)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM),
                ),
        )
}

fn is_error_toast(message: &str) -> bool {
    message.contains("失败")
        || message.contains("错误")
        || message.contains("未找到")
        || message.contains("Failed")
        || message.contains("Error")
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
        .bg(theme.panel_surface)
        .border(px(1.0))
        .border_color(theme.divider)
        .items_center()
        .justify_center()
        .child(text(label).text_color(theme.text_1).text_xs())
}
