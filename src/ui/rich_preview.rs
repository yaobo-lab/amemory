//! Rich preview renderer for Knowledge Items (Markdown, Code, PlainText).

use nsh_core::prelude::*;

use super::theme::ClippiTheme;

/// Render rich content preview block
pub fn rich_preview_view(
    content: &str,
    theme: ClippiTheme,
) -> impl IntoElement {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let divider = theme.divider;

    let lines: Vec<String> = content.lines().take(50).map(|s| s.to_string()).collect();

    view()
        .w_full()
        .bg(card_bg)
        .rounded(px(6.0))
        .border(px(1.0))
        .border_color(divider)
        .p(px(8.0))
        .flex_col()
        .gap(px(2.0))
        .child(
            scroll()
                .w_full()
                .h_full()
                .child(
                    view()
                        .flex_col()
                        .gap(px(2.0))
                        .children(lines.into_iter().map(|line| {
                            text(line)
                                .text_color(text_1)
                                .text_xs()
                        })),
                ),
        )
}
