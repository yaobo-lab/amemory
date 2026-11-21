//! Compact Filter chips bar and right-aligned `SearchMode` dropdown selector.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::filters::SearchMode;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::ItemKind;
use crate::state::app::AppStore;

fn search_mode_label(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Hybrid => I18nKey::SearchModeHybrid.text(),
        SearchMode::Lexical => I18nKey::SearchModeLexical.text(),
        SearchMode::Semantic => I18nKey::SearchModeSemantic.text(),
    }
}

/// Render filter bar with category chips on the left and search-mode dropdown + tools on the right.
pub fn filter_bar_view(
    store: AppStore,
    theme: ClippiTheme,
    on_type_changed: impl Fn(Option<ItemKind>) + 'static + Copy,
    _on_mode_changed: impl Fn(SearchMode) + 'static + Copy,
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
    let show_search_mode_menu = store.show_search_mode_menu;

    let kinds = [
        (None, I18nKey::FilterAll, glyphs::ALL),
        (Some(ItemKind::Skill), I18nKey::FilterSkill, glyphs::SKILL),
        (Some(ItemKind::Book), I18nKey::FilterBook, glyphs::BOOK),
        (
            Some(ItemKind::Document),
            I18nKey::FilterDocument,
            glyphs::DOCUMENT,
        ),
        (
            Some(ItemKind::Memory),
            I18nKey::FilterMemory,
            glyphs::MEMORY,
        ),
        (
            Some(ItemKind::Script),
            I18nKey::FilterScript,
            glyphs::SCRIPT,
        ),
        (Some(ItemKind::Note), I18nKey::FilterNote, glyphs::NOTE),
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

    let toggle_mode_menu = move || {
        show_search_mode_menu.update(|open| *open = !*open);
    };

    view()
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .gap(px(4.0))
        // Row 1: Categories + Search Mode Dropdown + Tag & Type Tools
        .child(
            view()
                .h(px(24.0))
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
                                .children(kinds.into_iter().map(move |(k, key, icon)| {
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
                                                    format!("按「{}」分类筛选", key.text()),
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
                                                    text(move || store.tr(key))
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
                // Right controls: SearchMode Dropdown | Tag Sidebar | Tag Manage | Type Config
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .pl(px(6.0))
                        .child(when(
                            move || !filters.get().tags.is_empty(),
                            move || {
                                let count = filters.get().tags.len();
                                view()
                                    .h(px(22.0))
                                    .px(px(6.0))
                                    .rounded(px(5.0))
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
                                        text(format!("已选 {count} ✕"))
                                            .text_color(accent)
                                            .text_xs(),
                                    )
                            },
                        ))
                        // Search Retrieval Mode Dropdown Trigger
                        .child(
                            view()
                                .h(px(22.0))
                                .px(px(8.0))
                                .rounded(px(5.0))
                                .bg(move || {
                                    if show_search_mode_menu.get() {
                                        accent_soft
                                    } else {
                                        surface
                                    }
                                })
                                .border(px(1.0))
                                .border_color(move || {
                                    if show_search_mode_menu.get() {
                                        accent
                                    } else {
                                        divider
                                    }
                                })
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .cursor_pointer()
                                .on_press(toggle_mode_menu)
                                .tooltip(tooltip_box("切换检索模式（混合 / 词法 / 语义）", theme))
                                .child(
                                    text(move || search_mode_label(filters.get().search_mode))
                                        .text_color(accent)
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_xs(),
                                )
                                .child(
                                    text(move || {
                                        if show_search_mode_menu.get() {
                                            "▴"
                                        } else {
                                            "▾"
                                        }
                                    })
                                    .text_color(text_3)
                                    .text_xs(),
                                ),
                        )
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
                                .h(px(22.0))
                                .px(px(6.0))
                                .rounded(px(5.0))
                                .bg(surface)
                                .border(px(1.0))
                                .border_color(divider)
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(open_tag_modal)
                                .tooltip(tooltip_box("打开标签过滤与新建面板", theme))
                                .child(text("标签管理").text_color(text_2).text_xs()),
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
}

/// Render animated dropdown menu overlay for selecting `SearchMode`.
pub fn search_mode_dropdown_overlay(
    store: AppStore,
    theme: ClippiTheme,
    on_mode_changed: impl Fn(SearchMode) + 'static + Copy,
) -> impl IntoElement {
    let filters = store.filters;
    let show_search_mode_menu = store.show_search_mode_menu;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;

    let mut timing = KeyframeTiming::default();
    timing.duration_ms = 170.0;
    timing.easing = Easing::Bezier(0.16, 1.0, 0.3, 1.0);
    let menu_motion = motion().entering(
        MotionSpec::new(timing)
            .values(MotionChannel::Opacity, [0.0, 1.0])
            .values(MotionChannel::Scale, [0.95, 1.0])
            .values(MotionChannel::TranslateY, [-6.0, 0.0]),
    );

    let options = [
        (
            SearchMode::Hybrid,
            I18nKey::SearchModeHybrid,
            "词法 + 语义综合",
        ),
        (
            SearchMode::Lexical,
            I18nKey::SearchModeLexical,
            "精确关键词匹配",
        ),
        (
            SearchMode::Semantic,
            I18nKey::SearchModeSemantic,
            "向量语义召回",
        ),
    ];

    view()
        .absolute()
        .w_full()
        .h_full()
        // Transparent click-outside backdrop
        .child(view().absolute().w_full().h_full().on_press(move || {
            show_search_mode_menu.set(false);
        }))
        // Dropdown panel aligned right beneath the mode selector
        .child(
            view()
                .absolute()
                .top(px(90.0))
                .right(px(120.0))
                .w(px(152.0))
                .p(px(5.0))
                .rounded(px(8.0))
                .bg(card_bg)
                .border(px(1.0))
                .border_color(divider)
                .shadow_xl()
                .flex_col()
                .gap(px(2.0))
                .bind_motion(&menu_motion)
                .children(options.into_iter().map(move |(mode, key, desc)| {
                    view()
                        .w_full()
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .bg(move || {
                            if filters.get().search_mode == mode {
                                accent_soft
                            } else {
                                card_bg
                            }
                        })
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .on_press(move || {
                            filters.update(|f| f.search_mode = mode);
                            show_search_mode_menu.set(false);
                            on_mode_changed(mode);
                        })
                        .child(
                            view()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    text(key.text())
                                        .text_color(move || {
                                            if filters.get().search_mode == mode {
                                                accent
                                            } else {
                                                text_1
                                            }
                                        })
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_xs(),
                                )
                                .child(text(desc).text_color(text_3).text_size(px(10.0))),
                        )
                        .child(when(
                            move || filters.get().search_mode == mode,
                            move || {
                                text("✓")
                                    .text_color(accent)
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs()
                            },
                        ))
                })),
        )
}
