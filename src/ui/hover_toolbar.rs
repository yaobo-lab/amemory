//! Card action hover toolbar extending from card top-right.

use nsh_core::MouseEvent;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;

/// Render 20px action tab toolbar for a card.
#[allow(clippy::too_many_arguments)]
pub fn hover_toolbar_view(
    is_favorite: impl Fn() -> bool + Copy + 'static,
    theme: ClippiTheme,
    on_copy: impl Fn() + 'static,
    on_tag: impl Fn() + 'static,
    on_more: impl Fn() + 'static,
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
                I18nKey::CardCopyContent.text(),
                theme,
                surface,
                accent,
                move || text_2,
                divider,
                on_copy,
            ),
        )
        // 2. Manage Tags
        .child(
            render_tab(
                glyphs::TAG,
                I18nKey::CardManageTags.text(),
                theme,
                surface,
                accent,
                move || text_2,
                divider,
                on_tag,
            ),
        )
        // 3. More / Context Menu (Copy URI, Open Dir, etc.)
        .child(
            render_tab(
                glyphs::FOLDER,
                "更多操作 (复制URI / 打开目录)",
                theme,
                surface,
                accent,
                move || text_2,
                divider,
                on_more,
            ),
        )
        // 4. Edit
        .child(
            render_tab(
                glyphs::EDIT,
                I18nKey::CardEditItem.text(),
                theme,
                surface,
                accent,
                move || text_2,
                divider,
                on_edit,
            ),
        )
        // 5. Favorite
        .child(
            render_tab(
                glyphs::STAR,
                I18nKey::CardFavorite.text(),
                theme,
                surface,
                fav_color,
                move || if is_favorite() { fav_color } else { text_2 },
                divider,
                on_fav,
            ),
        )
        // 6. Delete
        .child(
            render_tab(
                glyphs::DELETE,
                I18nKey::CardDelete.text(),
                theme,
                surface,
                danger,
                move || text_2,
                divider,
                on_delete,
            ),
        )
}

#[allow(clippy::too_many_arguments, clippy::large_types_passed_by_value)]
fn render_tab(
    icon: &'static str,
    tip: &'static str,
    theme: ClippiTheme,
    bg: Rgba,
    hover_bg: Rgba,
    text_color: impl Fn() -> Rgba + Copy + 'static,
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
        .tooltip(tooltip_box(tip, theme))
        .child(
            text(icon)
                .font_family("iconfont")
                .text_color(move || if is_hovered.get() { rgb(0xffffff) } else { text_color() })
                .text_xs(),
        )
}
