//! Dual-row 22px Filter chips and `SearchMode` selectors matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::filters::SearchMode;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::ItemKind;
use crate::state::app::AppStore;

/// Render dual-row filter bar.
pub fn filter_bar_view(
    store: AppStore,
    theme: ClippiTheme,
    on_type_changed: impl Fn(Option<ItemKind>) + 'static + Copy,
    on_mode_changed: impl Fn(SearchMode) + 'static + Copy,
) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;

    let filters = store.filters;
    let settings = store.settings;
    let is_sidebar_open = store.is_sidebar_open;
    let show_tag_filter = store.show_tag_filter;
    let show_type_config = store.show_type_config;

    let kinds = [
        (None, I18nKey::FilterAll.text(), glyphs::ALL),
        (
            Some(ItemKind::Skill),
            I18nKey::FilterSkill.text(),
            glyphs::SKILL,
        ),
        (
            Some(ItemKind::Book),
            I18nKey::FilterBook.text(),
            glyphs::BOOK,
        ),
        (
            Some(ItemKind::Document),
            I18nKey::FilterDocument.text(),
            glyphs::DOCUMENT,
        ),
        (
            Some(ItemKind::Memory),
            I18nKey::FilterMemory.text(),
            glyphs::MEMORY,
        ),
        (
            Some(ItemKind::Script),
            I18nKey::FilterScript.text(),
            glyphs::SCRIPT,
        ),
        (
            Some(ItemKind::Note),
            I18nKey::FilterNote.text(),
            glyphs::NOTE,
        ),
    ];

    let toggle_sidebar = move || {
        is_sidebar_open.update(|v| *v = !*v);
    };

    let open_tag_modal = move || {
        show_tag_filter.set(true);
    };

    let open_type_modal = move || {
        show_type_config.set(true);
    };

    view()
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .gap(px(4.0))
        // Row 1: Categories + Tag Sidebar + Type Config Button
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
                                    when(
                                        move || {
                                            k.is_none_or(|kind_val| {
                                                settings.get().is_kind_visible(kind_val)
                                            })
                                        },
                                        move || {
                                            view()
                                                .h(px(22.0))
                                                .px(px(6.0))
                                                .rounded(px(5.0))
                                                .bg(move || {
                                                    if filters.get().kind == k {
                                                        accent_soft
                                                    } else {
                                                        surface
                                                    }
                                                })
                                                .border(px(1.0))
                                                .border_color(move || {
                                                    if filters.get().kind == k {
                                                        accent
                                                    } else {
                                                        divider
                                                    }
                                                })
                                                .flex_row()
                                                .items_center()
                                                .gap(px(4.0))
                                                .cursor_pointer()
                                                .on_press(move || {
                                                    let mut next_k = k;
                                                    filters.update(|f| {
                                                        if f.kind == k && k.is_some() {
                                                            f.kind = None;
                                                            next_k = None;
                                                        } else {
                                                            f.kind = k;
                                                        }
                                                    });
                                                    on_type_changed(next_k);
                                                })
                                                .tooltip(tooltip_box(
                                                    format!("按「{label}」分类筛选"),
                                                    theme,
                                                ))
                                                .child(
                                                    text(icon)
                                                        .font_family("iconfont")
                                                        .text_color(move || {
                                                            if filters.get().kind == k {
                                                                accent
                                                            } else {
                                                                text_2
                                                            }
                                                        })
                                                        .text_xs(),
                                                )
                                                .child(
                                                    text(label)
                                                        .text_color(move || {
                                                            if filters.get().kind == k {
                                                                accent
                                                            } else {
                                                                text_2
                                                            }
                                                        })
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_xs(),
                                                )
                                        },
                                    )
                                })),
                        ),
                )
                // Tag sidebar toggle + Type filter config button
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
                                .bg(move || {
                                    if is_sidebar_open.get() || !filters.get().tags.is_empty() {
                                        accent_soft
                                    } else {
                                        surface
                                    }
                                })
                                .border(px(1.0))
                                .border_color(move || {
                                    if is_sidebar_open.get() || !filters.get().tags.is_empty() {
                                        accent
                                    } else {
                                        divider
                                    }
                                })
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(toggle_sidebar)
                                .tooltip(tooltip_box(I18nKey::FilterTagButton.text(), theme))
                                .child(
                                    text(glyphs::TAG)
                                        .font_family("iconfont")
                                        .text_color(move || {
                                            if is_sidebar_open.get()
                                                || !filters.get().tags.is_empty()
                                            {
                                                accent
                                            } else {
                                                text_2
                                            }
                                        })
                                        .text_xs(),
                                ),
                        )
                        .child(
                            view()
                                .w(px(24.0))
                                .h(px(22.0))
                                .rounded(px(5.0))
                                .bg(surface)
                                .border(px(1.0))
                                .border_color(divider)
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(open_type_modal)
                                .tooltip(tooltip_box(I18nKey::SettingFilterTypes.text(), theme))
                                .child(
                                    text(glyphs::SETTINGS)
                                        .font_family("iconfont")
                                        .text_color(text_2)
                                        .text_xs(),
                                ),
                        ),
                ),
        )
        // Row 2: Search Retrieval Mode (Hybrid / Lexical / Semantic) + Tag Management & Active Filter Reset
        .child(
            view()
                .h(px(20.0))
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(render_mode_chip(
                            SearchMode::Hybrid,
                            I18nKey::SearchModeHybrid.text(),
                            filters,
                            theme,
                            on_mode_changed,
                        ))
                        .child(render_mode_chip(
                            SearchMode::Lexical,
                            I18nKey::SearchModeLexical.text(),
                            filters,
                            theme,
                            on_mode_changed,
                        ))
                        .child(render_mode_chip(
                            SearchMode::Semantic,
                            I18nKey::SearchModeSemantic.text(),
                            filters,
                            theme,
                            on_mode_changed,
                        )),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(when(
                            move || !filters.get().tags.is_empty(),
                            move || {
                                let count = filters.get().tags.len();
                                view()
                                    .h(px(18.0))
                                    .px(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(accent_soft)
                                    .border(px(1.0))
                                    .border_color(accent)
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .on_press(move || {
                                        filters.update(|f| f.tags.clear());
                                    })
                                    .tooltip(tooltip_box("点击清空已选标签过滤", theme))
                                    .child(
                                        text(format!("已选 {count} 标签 ✕"))
                                            .text_color(accent)
                                            .text_xs(),
                                    )
                            },
                        ))
                        .child(
                            view()
                                .h(px(18.0))
                                .px(px(6.0))
                                .rounded(px(4.0))
                                .bg(surface)
                                .border(px(1.0))
                                .border_color(divider)
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(open_tag_modal)
                                .tooltip(tooltip_box("打开标签过滤与新建面板", theme))
                                .child(text("🏷️ 标签管理").text_color(text_3).text_xs()),
                        ),
                ),
        )
}

#[allow(clippy::large_types_passed_by_value)]
fn render_mode_chip(
    mode: SearchMode,
    label: &'static str,
    filters: Signal<crate::core::filters::FilterCriteria>,
    theme: ClippiTheme,
    on_mode_changed: impl Fn(SearchMode) + 'static + Copy,
) -> impl IntoElement {
    let accent_soft = theme.accent_soft;
    let surface = theme.surface;
    let accent = theme.accent;
    let divider = theme.divider;

    view()
        .h(px(18.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(move || {
            if filters.get().search_mode == mode {
                accent_soft
            } else {
                surface
            }
        })
        .border(px(1.0))
        .border_color(move || {
            if filters.get().search_mode == mode {
                accent
            } else {
                divider
            }
        })
        .items_center()
        .justify_center()
        .cursor_pointer()
        .on_press(move || {
            filters.update(|f| f.search_mode = mode);
            on_mode_changed(mode);
        })
        .child(
            text(label)
                .text_color(move || {
                    if filters.get().search_mode == mode {
                        theme.accent
                    } else {
                        theme.text_3
                    }
                })
                .text_xs(),
        )
}
