//! Modal confirmation dialog (240px width, rounded 10px).

use nsh_core::prelude::*;

use super::modal_overlay::modal_overlay_view;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::ConfirmDialogData;
use crate::ui::theme::ClippiTheme;

/// Render confirmation dialog modal box.
pub fn confirm_dialog_box(
    data: ConfirmDialogData,
    theme: ClippiTheme,
    on_confirm: impl Fn() + 'static,
    on_cancel: impl Fn() + 'static,
) -> impl IntoElement {
    confirm_dialog_view(data.title, data.message, theme, on_confirm, on_cancel)
}

/// Render confirmation dialog view.
pub fn confirm_dialog_view(
    title: impl Into<String>,
    message: impl Into<String>,
    theme: ClippiTheme,
    on_confirm: impl Fn() + 'static,
    on_cancel: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    let message = message.into();
    let surface = theme.surface;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let danger = theme.danger;

    modal_overlay_view(
        view()
            .w(px(240.0))
            .p(px(14.0))
            .rounded(px(10.0))
            .bg(surface)
            .border(px(1.0))
            .border_color(divider)
            .flex_col()
            .gap(px(10.0))
            .child(
                text(title)
                    .text_color(text_1)
                    .font_weight(FontWeight::BOLD)
                    .text_xs(),
            )
            .child(
                text(message)
                    .text_color(text_2)
                    .text_xs(),
            )
            .child(
                view()
                    .flex_row()
                    .justify_end()
                    .gap(px(8.0))
                    .pt(px(6.0))
                    .child(
                        view()
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(divider)
                            .cursor_pointer()
                            .on_press(on_cancel)
                            .child(
                                text(I18nKey::DialogCancel.text())
                                    .text_color(text_2)
                                    .text_xs(),
                            ),
                    )
                    .child(
                        view()
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(danger)
                            .cursor_pointer()
                            .on_press(on_confirm)
                            .child(
                                text(I18nKey::DialogConfirm.text())
                                    .text_color(rgb(0xffffff))
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                            ),
                    ),
            ),
        theme,
        move || {},
    )
}
