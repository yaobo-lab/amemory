//! 56px Attached left tag sidebar matching Clippi 1:1.

use nsh_core::prelude::*;

use super::theme::ClippiTheme;
use crate::state::app::AppStore;

/// Render 56px left tag sidebar view.
pub fn sidebar_view(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let tag_bg = theme.tag_bg;
    let tag_text = theme.tag_text;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;

    let filters = store.filters;

    // Common tags from catalog & system
    let tag_list = vec![
        "全部", "rust", "mcp", "xerj", "book", "skill", "doc", "async", "tokio",
    ];

    view()
        .w(px(56.0))
        .h_full()
        .bg(surface)
        .border_r(px(1.0))
        .border_color(divider)
        .p(px(4.0))
        .child(
            scroll()
                .flex_1()
                .child(
                    view()
                        .flex_col()
                        .gap(px(2.0))
                        .children(tag_list.into_iter().map(move |tag_name| {
                            let tag_bg_str = tag_name.to_string();
                            let tag_border_str = tag_name.to_string();
                            let tag_text_str = tag_name.to_string();
                            let tag_click = tag_name.to_string();

                            view()
                                .h(px(22.0))
                                .w_full()
                                .rounded(px(5.0))
                                .bg(move || if filters.get().tags.contains(&tag_bg_str) { accent_soft } else { tag_bg })
                                .border(px(1.0))
                                .border_color(move || if filters.get().tags.contains(&tag_border_str) { accent } else { divider })
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(move || {
                                    filters.update(|f| {
                                        if let Some(pos) = f.tags.iter().position(|t| t == &tag_click) {
                                            f.tags.remove(pos);
                                        } else {
                                            f.tags.push(tag_click.clone());
                                        }
                                    });
                                })
                                .child(
                                    text(tag_name)
                                        .text_color(move || if filters.get().tags.contains(&tag_text_str) { accent } else { tag_text })
                                        .text_xs(),
                                )
                        })),
                ),
        )
}
