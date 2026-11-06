//! Dual-row 22px Filter chips and SearchMode selectors matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::core::filters::SearchMode;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::ItemKind;
use crate::state::app::AppStore;

/// Render dual-row filter bar.
pub fn filter_bar_view(
    store: AppStore,
    theme: ClippiTheme,
    _on_type_changed: impl Fn(Option<ItemKind>) + 'static + Copy,
    _on_mode_changed: impl Fn(SearchMode) + 'static + Copy,
) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let text_2 = theme.text_2;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;

    let filters = store.filters;
    let is_sidebar_open = store.is_sidebar_open;

    let kinds = [
        (None, I18nKey::FilterAll.text(), glyphs::ALL),
        (Some(ItemKind::Skill), I18nKey::FilterSkill.text(), glyphs::SKILL),
        (Some(ItemKind::Book), I18nKey::FilterBook.text(), glyphs::BOOK),
        (Some(ItemKind::Document), I18nKey::FilterDocument.text(), glyphs::DOCUMENT),
        (Some(ItemKind::Memory), I18nKey::FilterMemory.text(), glyphs::MEMORY),
        (Some(ItemKind::Script), I18nKey::FilterScript.text(), glyphs::SCRIPT),
        (Some(ItemKind::Note), I18nKey::FilterNote.text(), glyphs::NOTE),
    ];

    let toggle_sidebar = move || {
        is_sidebar_open.update(|v| *v = !*v);
    };

    view()
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .gap(px(4.0))
        // Row 1: Categories + Tag Button
        .child(
            view()
                .h(px(22.0))
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    scroll()
                        .horizontal()
                        .flex_1()
                        .child(
                            view()
                                .flex_row()
                                .gap(px(4.0))
                                .children(kinds.into_iter().map(move |(k, label, icon)| {
                                    view()
                                        .h(px(22.0))
                                        .px(px(6.0))
                                        .rounded(px(5.0))
                                        .bg(move || if filters.get().kind == k { accent_soft } else { surface })
                                        .border(px(1.0))
                                        .border_color(move || if filters.get().kind == k { accent } else { divider })
                                        .flex_row()
                                        .items_center()
                                        .gap(px(4.0))
                                        .cursor_pointer()
                                        .on_press(move || {
                                            filters.update(|f| f.kind = k);
                                        })
                                        .child(
                                            text(icon)
                                                .text_color(move || if filters.get().kind == k { accent } else { text_2 })
                                                .text_xs(),
                                        )
                                        .child(
                                            text(label)
                                                .text_color(move || if filters.get().kind == k { accent } else { text_2 })
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_xs(),
                                        )
                                })),
                        ),
                )
                // Tag sidebar toggle button
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .pl(px(4.0))
                        .child(view().w(px(1.0)).h(px(14.0)).bg(divider))
                        .child(
                            view()
                                .w(px(24.0))
                                .h(px(22.0))
                                .rounded(px(5.0))
                                .bg(move || if is_sidebar_open.get() { accent_soft } else { surface })
                                .border(px(1.0))
                                .border_color(move || if is_sidebar_open.get() { accent } else { divider })
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(toggle_sidebar)
                                .child(
                                    text(glyphs::TAG)
                                        .text_color(move || if is_sidebar_open.get() { accent } else { text_2 })
                                        .text_xs(),
                                ),
                        ),
                ),
        )
        // Row 2: Search Retrieval Mode (Hybrid / Lexical / Semantic)
        .child(
            view()
                .h(px(20.0))
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(render_mode_chip(
                    SearchMode::Hybrid,
                    I18nKey::SearchModeHybrid.text(),
                    filters,
                    theme,
                ))
                .child(render_mode_chip(
                    SearchMode::Lexical,
                    I18nKey::SearchModeLexical.text(),
                    filters,
                    theme,
                ))
                .child(render_mode_chip(
                    SearchMode::Semantic,
                    I18nKey::SearchModeSemantic.text(),
                    filters,
                    theme,
                )),
        )
}

fn render_mode_chip(
    mode: SearchMode,
    label: &'static str,
    filters: Signal<crate::core::filters::FilterCriteria>,
    theme: ClippiTheme,
) -> impl IntoElement {
    view()
        .h(px(18.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(move || if filters.get().search_mode == mode { theme.accent_soft } else { theme.surface })
        .border(px(1.0))
        .border_color(move || if filters.get().search_mode == mode { theme.accent } else { theme.divider })
        .items_center()
        .justify_center()
        .cursor_pointer()
        .on_press(move || {
            filters.update(|f| f.search_mode = mode);
        })
        .child(
            text(label)
                .text_color(move || if filters.get().search_mode == mode { theme.accent } else { theme.text_3 })
                .text_xs(),
        )
}
