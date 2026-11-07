//! Type filter configuration dialog.

use nsh_core::prelude::*;

use super::components::modal_overlay::modal_overlay_view;
use super::components::toggle::render_toggle;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::ItemKind;
use crate::state::app::AppStore;

/// Render type filter configuration dialog
pub fn type_filter_config_view(
    store: AppStore,
    theme: ClippiTheme,
    on_close: impl Fn() + 'static + Copy,
) -> impl IntoElement {
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;
    let accent = theme.accent;

    modal_overlay_view(
        view()
            .w(px(280.0))
            .bg(card_bg)
            .rounded(px(10.0))
            .border(px(1.0))
            .border_color(divider)
            .p(px(14.0))
            .flex_col()
            .gap(px(10.0))
            // Header
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        text(I18nKey::SettingFilterTypes.text())
                            .text_color(text_1)
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                    )
                    .child(
                        view()
                            .cursor_pointer()
                            .on_press(move || {
                                on_close();
                            })
                            .child(
                                text("✕")
                                    .text_color(text_3)
                                    .text_xs(),
                            ),
                    ),
            )
            // Description
            .child(
                text("选择在顶部过滤栏显示的知识类型快捷标签:")
                    .text_color(text_3)
                    .text_xs(),
            )
            // Type toggle rows
            .child(type_toggle_row(store, ItemKind::Skill, "Skills 技能", "Prompt 模板与执行动作", theme))
            .child(type_toggle_row(store, ItemKind::Book, "Books 书本", "知识书本与参考手册", theme))
            .child(type_toggle_row(store, ItemKind::Document, "Docs 文档", "长文档与分段章节", theme))
            .child(type_toggle_row(store, ItemKind::Memory, "Memory 记忆", "XERJ 实时记忆片段", theme))
            .child(type_toggle_row(store, ItemKind::Script, "Scripts 脚本", "自动化脚本与工具代码", theme))
            .child(type_toggle_row(store, ItemKind::Note, "Notes 便签", "临时笔记与参考文档", theme))
            // Bottom Done button
            .child(
                view()
                    .w_full()
                    .items_end()
                    .pt(px(6.0))
                    .child(
                        view()
                            .px(px(14.0))
                            .py(px(4.0))
                            .rounded(px(5.0))
                            .bg(accent)
                            .cursor_pointer()
                            .on_press(move || {
                                on_close();
                            })
                            .child(
                                text(I18nKey::DialogConfirm.text())
                                    .text_color(rgb(0xffffff))
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                            ),
                    ),
            ),
        theme,
        on_close,
    )
}

#[allow(clippy::large_types_passed_by_value)]
fn type_toggle_row(
    store: AppStore,
    kind: ItemKind,
    label: &'static str,
    desc: &'static str,
    theme: ClippiTheme,
) -> impl IntoElement {
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;
    let initial_enabled = store.settings.get_untracked().is_kind_visible(kind);

    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(3.0))
        .child(
            view()
                .flex_col()
                .child(
                    text(label)
                        .text_color(text_1)
                        .text_xs(),
                )
                .child(
                    text(desc)
                        .text_color(text_3)
                        .text_xs(),
                ),
        )
        .child(
            render_toggle(
                initial_enabled,
                theme,
                move |next| {
                    store.update_settings(|s| s.set_kind_visible(kind, next));
                    if !next && store.filters.get_untracked().kind == Some(kind) {
                        store.filters.update(|f| f.kind = None);
                    }
                },
            ),
        )
}
