//! 28px Search box with expandable "+" new item button.

use nsh_base::input;
use nsh_core::MouseEvent;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::{ActiveView, AppStore};

/// Render 28px search box and new item action button.
pub fn search_box_view(
    store: AppStore,
    theme: ClippiTheme,
    on_search: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;

    let query_signal = store.search_query;
    let is_expanded = signal(false);

    let open_new_item = move || {
        store.active_view.set(ActiveView::Edit);
    };

    view()
        .h(px(28.0))
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        // Search Input container
        .child(
            view()
                .flex_1()
                .h(px(28.0))
                .rounded(px(10.0))
                .bg(surface)
                .border(px(1.0))
                .border_color(divider)
                .flex_row()
                .items_center()
                .px(px(8.0))
                .gap(px(6.0))
                .child(
                    text(glyphs::SEARCH)
                        .text_color(text_3)
                        .text_sm(),
                )
                .child(
                    input()
                        .value(move || query_signal.get())
                        .on_change(move |val: &str| {
                            query_signal.set(val.to_string());
                            on_search(val.to_string());
                        })
                        .placeholder(I18nKey::SearchPlaceholder.text())
                        .flex_1()
                        .h(px(26.0))
                        .text_color(move || text_1)
                        .text_xs(),
                ),
        )
        // Expandable "+" Button
        .child(
            view()
                .h(px(28.0))
                .rounded(px(10.0))
                .bg(move || if is_expanded.get() { accent } else { surface })
                .border(px(1.0))
                .border_color(move || if is_expanded.get() { accent } else { divider })
                .flex_row()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .px(px(8.0))
                .gap(px(4.0))
                .on_mouse_enter(move |_: &MouseEvent| is_expanded.set(true))
                .on_mouse_leave(move |_: &MouseEvent| is_expanded.set(false))
                .on_press(open_new_item)
                .style_with(move |s| {
                    if is_expanded.get() {
                        s.w(px(76.0))
                    } else {
                        s.w(px(28.0))
                    }
                })
                .child(
                    text(glyphs::PLUS)
                        .text_color(move || if is_expanded.get() { rgb(0xffffff) } else { text_2 })
                        .text_sm(),
                )
                .child(
                    when(
                        move || is_expanded.get(),
                        move || {
                            text(I18nKey::EditTitleNew.text())
                                .text_color(rgb(0xffffff))
                                .font_weight(FontWeight::BOLD)
                                .text_xs()
                        },
                    ),
                ),
        )
}
