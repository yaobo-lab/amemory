//! 26px Bottom status bar matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::state::app::AppStore;

/// Render 26px bottom status bar.
pub fn bottom_bar_view(
    store: AppStore,
    theme: ClippiTheme,
    on_quick_sync: impl Fn() + 'static,
    on_copy_endpoint: impl Fn() + 'static,
) -> impl IntoElement {
    let titlebar_bg = theme.titlebar_bg;
    let divider = theme.divider;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let btn_hover = theme.btn_hover;

    let stats = store.stats;

    view()
        .h(px(26.0))
        .w_full()
        .bg(titlebar_bg)
        .border_t(px(1.0))
        .border_color(divider)
        .px(px(10.0))
        .flex_row()
        .items_center()
        .justify_between()
        // Left: live status & count pills
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    view()
                        .size(px(6.0))
                        .rounded(px(3.0))
                        .bg(accent),
                )
                .child(
                    text(move || {
                        format!(
                            "Skills: {} · Books: {} · Docs: {}",
                            stats.with(|s| s.skills_count),
                            stats.with(|s| s.books_count),
                            stats.with(|s| s.total_docs),
                        )
                    })
                    .text_color(text_3)
                    .text_xs(),
                ),
        )
        // Right: quick action buttons (22x22)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    view()
                        .size(px(22.0))
                        .rounded(px(4.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(on_quick_sync)
                        .child(text(glyphs::SYNC).text_color(text_2).text_xs()),
                )
                .child(
                    view()
                        .size(px(22.0))
                        .rounded(px(4.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(on_copy_endpoint)
                        .child(text(glyphs::COPY).text_color(text_2).text_xs()),
                ),
        )
}
