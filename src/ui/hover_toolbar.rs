//! Card action hover toolbar extending from card top-right.

use nsh_core::MouseEvent;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::core::types::KnowledgeItem;

/// Render 20px action tab toolbar for a card.
pub fn hover_toolbar_view(
    item: KnowledgeItem,
    theme: ClippiTheme,
    on_copy: impl Fn() + 'static,
    on_edit: impl Fn() + 'static,
    on_fav: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let text_2 = theme.text_2;
    let accent = theme.accent;
    let danger = theme.danger;
    let fav_color = theme.fav_color;

    view()
        .absolute()
        .top_0()
        .right(px(8.0))
        .h(px(20.0))
        .flex_row()
        .gap(px(2.0))
        // 1. Copy
        .child(
            render_tab(
                glyphs::COPY,
                surface,
                accent,
                text_2,
                divider,
                on_copy,
            ),
        )
        // 2. Edit
        .child(
            render_tab(
                glyphs::EDIT,
                surface,
                accent,
                text_2,
                divider,
                on_edit,
            ),
        )
        // 3. Favorite
        .child(
            render_tab(
                glyphs::STAR,
                surface,
                fav_color,
                if item.favorite { fav_color } else { text_2 },
                divider,
                on_fav,
            ),
        )
        // 4. Delete
        .child(
            render_tab(
                glyphs::DELETE,
                surface,
                danger,
                text_2,
                divider,
                on_delete,
            ),
        )
}

fn render_tab(
    icon: &'static str,
    bg: Rgba,
    hover_bg: Rgba,
    text_color: Rgba,
    border_color: Rgba,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let is_hovered = signal(false);

    view()
        .w(px(20.0))
        .h(px(20.0))
        .rounded_b(px(4.0))
        .bg(move || if is_hovered.get() { hover_bg } else { bg })
        .border(px(1.0))
        .border_color(border_color)
        .items_center()
        .justify_center()
        .cursor_pointer()
        .on_mouse_enter(move |_: &MouseEvent| is_hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| is_hovered.set(false))
        .on_press(on_press)
        .child(
            text(icon)
                .text_color(move || if is_hovered.get() { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}
