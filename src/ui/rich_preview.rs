//! Rich preview renderer for Knowledge Items (Markdown, Code, `PlainText`).

use nsh_base::text_view;
use nsh_core::prelude::*;

use super::theme::ClippiTheme;

/// Render rich Markdown preview block using NSH's native `TextView`.
#[must_use]
pub fn rich_preview_view(content: &str, theme: ClippiTheme) -> impl IntoElement + use<> {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let markdown = SharedString::from(content.to_owned());

    view()
        .w_full()
        .flex_1()
        .bg(card_bg)
        .rounded(px(6.0))
        .border(px(1.0))
        .border_color(divider)
        .overflow_hidden()
        .child(
            scroll().w_full().h_full().p(px(10.0)).child(
                text_view()
                    .markdown(markdown)
                    .selectable(true)
                    .w_full()
                    .text_color(text_1),
            ),
        )
}
