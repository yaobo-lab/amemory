//! 184px Right-click context menu matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;

/// Render 184px right-click context menu box.
pub fn context_menu_box(
    theme: ClippiTheme,
    on_copy: impl Fn() + 'static,
    on_copy_uri: impl Fn() + 'static,
    on_edit: impl Fn() + 'static,
    on_fav: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let btn_hover = theme.btn_hover;
    let text_1 = theme.text_1;
    let danger = theme.danger;

    view()
        .w(px(184.0))
        .p(px(4.0))
        .rounded(px(8.0))
        .bg(surface)
        .border_1()
        .border_color(divider)
        .gap(px(2.0))
        .child(render_menu_item(glyphs::COPY, I18nKey::CardCopyContent.text(), "Ctrl+C", text_1, btn_hover, on_copy))
        .child(render_menu_item(glyphs::TAG, I18nKey::CardCopyUri.text(), "Ctrl+U", text_1, btn_hover, on_copy_uri))
        .child(view().h(px(1.0)).my(px(2.0)).bg(divider))
        .child(render_menu_item(glyphs::EDIT, I18nKey::CardEditItem.text(), "Ctrl+E", text_1, btn_hover, on_edit))
        .child(render_menu_item(glyphs::STAR, I18nKey::CardFavorite.text(), "Ctrl+F", text_1, btn_hover, on_fav))
        .child(view().h(px(1.0)).my(px(2.0)).bg(divider))
        .child(render_menu_item(glyphs::DELETE, I18nKey::CardDelete.text(), "Del", danger, btn_hover, on_delete))
}

fn render_menu_item(
    icon: &'static str,
    label: &'static str,
    shortcut: &'static str,
    text_color: Rgba,
    hover_bg: Rgba,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .h(px(30.0))
        .px(px(8.0))
        .rounded(px(5.0))
        .flex_row()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_press(on_press)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(text(icon).text_color(text_color).text_sm())
                .child(text(label).text_color(text_color).text_xs()),
        )
        .child(
            text(shortcut)
                .text_color(text_color)
                .text_xs(),
        )
}
