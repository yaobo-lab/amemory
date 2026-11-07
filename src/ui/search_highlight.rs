//! Search highlight text builder.

use nsh_core::prelude::*;

use crate::core::search::highlight_text;

/// Render text with query matches highlighted.
#[must_use]
pub fn search_highlight_view(
    content: &str,
    query: &str,
    normal_color: nsh_core::prelude::Rgba,
    highlight_color: nsh_core::prelude::Rgba,
    highlight_bg: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    if query.trim().is_empty() {
        return view().child(
            text(content.to_string())
                .text_color(normal_color)
                .text_xs(),
        );
    }

    let spans = highlight_text(content, query);
    let mut row = view().flex_row().flex_wrap();

    for span in spans {
        if span.is_highlight {
            row = row.child(
                view()
                    .bg(highlight_bg)
                    .rounded(px(2.0))
                    .px(px(2.0))
                    .child(
                        text(span.text)
                            .text_color(highlight_color)
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                    ),
            );
        } else {
            row = row.child(
                text(span.text)
                    .text_color(normal_color)
                    .text_xs(),
            );
        }
    }

    row
}
