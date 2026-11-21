//! Home dashboard statistics and XERJ engine metric banner.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::assets::kind_icons;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::{HealthLevel, ItemKind, StatsSnapshot};
use crate::state::app::AppStore;

/// Height of each asset summary card.
const STAT_CARD_HEIGHT: f32 = 56.0;
/// Side of the tinted icon tile inside each summary card.
const STAT_TILE_SIZE: f32 = 34.0;
/// Side of the SVG icon inside each summary card's tile.
const STAT_ICON_SIZE: f32 = 18.0;

/// Render home stats overview cards and XERJ metric bar.
#[must_use]
pub fn stats_overview_view(store: AppStore, _theme: ClippiTheme) -> impl IntoElement {
    let stats = store.stats;

    view()
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .gap(px(6.0))
        // 1. Three-card asset summary grid
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(stat_card(
                    store,
                    ItemKind::Skill,
                    I18nKey::StatsSkills,
                    "点击筛选全部技能 (Skills)",
                    move || stats.with(|s| s.skills_count.to_string()),
                    move || {
                        stats.with(|s| {
                            format!("{} 分片 · {} 文件", s.skills_docs, s.skill_files_count)
                        })
                    },
                ))
                .child(stat_card(
                    store,
                    ItemKind::Book,
                    I18nKey::StatsBooks,
                    "点击筛选全部书本 (Books)",
                    move || stats.with(|s| s.books_count.to_string()),
                    move || {
                        stats.with(|s| {
                            format!("{} 分片 · {} 导入", s.books_docs, s.imported_docs_count)
                        })
                    },
                ))
                .child(stat_card(
                    store,
                    ItemKind::Memory,
                    I18nKey::StatsMemory,
                    "点击筛选全部记忆 (Memory)",
                    move || stats.with(|s| s.memory_docs.to_string()),
                    move || stats.with(|s| format!("{} 个命名空间", s.namespace_count)),
                )),
        )
        // 2. XERJ engine metric bar
        .child(
            view()
                .h(px(28.0))
                .w_full()
                .rounded(px(8.0))
                .bg(move || store.theme().surface)
                .border_1()
                .border_color(move || store.theme().divider)
                .px(px(10.0))
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        // Engine health badge
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(5.0))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(5.0))
                                .bg(move || store.theme().tag_bg)
                                .child(view().size(px(6.0)).rounded(px(3.0)).bg(move || {
                                    let t = store.theme();
                                    match stats.with(|s| s.health) {
                                        HealthLevel::Green => t.accent,
                                        HealthLevel::Yellow => t.fav_color,
                                        HealthLevel::Red => t.danger,
                                        HealthLevel::Unknown => t.text_3,
                                    }
                                }))
                                .child(
                                    text("XERJ")
                                        .text_color(move || store.theme().text_1)
                                        .font_weight(FontWeight::BOLD)
                                        .text_size(px(10.5)),
                                ),
                        )
                        // Structured metrics: Docs | Indices | Disk
                        .child(metric_pair(store, "文档", move || {
                            stats.with(|s| s.total_docs.to_string())
                        }))
                        .child(metric_divider(store))
                        .child(metric_pair(store, "索引库", move || {
                            stats.with(|s| s.index_count.to_string())
                        }))
                        .child(metric_divider(store))
                        .child(metric_pair(store, "磁盘", move || {
                            stats.with(StatsSnapshot::disk_usage_label)
                        })),
                )
                // Right latency / embed mode pill
                .child(
                    view()
                        .px(px(7.0))
                        .py(px(2.0))
                        .rounded(px(5.0))
                        .bg(move || store.theme().accent_soft)
                        .child(
                            text(move || {
                                stats.with(|s| s.last_search_latency_ms).map_or_else(
                                    || stats.with(|s| s.embed_mode.clone()),
                                    |ms| format!("{ms} ms"),
                                )
                            })
                            .text_color(move || store.theme().accent)
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(10.5)),
                        ),
                ),
        )
}

/// Single asset summary card with tinted Lucide icon tile and clear count hierarchy.
fn stat_card(
    store: AppStore,
    kind: ItemKind,
    title_key: I18nKey,
    tooltip_text: &'static str,
    count_fn: impl Fn() -> String + 'static,
    detail_fn: impl Fn() -> String + 'static,
) -> impl IntoElement {
    let filters = store.filters;
    let hue = kind_icons::hue(kind);
    let icon_src = ImageSrc::svg_markup(kind_icons::markup(kind));
    let is_active = move || filters.get().kind == Some(kind);

    let toggle_filter = move || {
        filters.update(|f| {
            f.kind = if f.kind == Some(kind) {
                None
            } else {
                Some(kind)
            };
        });
    };

    view()
        .flex_1()
        .h(px(STAT_CARD_HEIGHT))
        .rounded(px(10.0))
        .bg(move || {
            if is_active() {
                rgba((hue << 8) | 0x16)
            } else {
                store.theme().surface
            }
        })
        .border_1()
        .border_color(move || {
            if is_active() {
                rgb(hue)
            } else {
                store.theme().divider
            }
        })
        .px(px(10.0))
        .py(px(8.0))
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .cursor_pointer()
        .active(|s| s.opacity(0.75))
        .on_press(toggle_filter)
        .tooltip(tooltip_box(tooltip_text, store.theme()))
        // Tinted Lucide icon tile
        .child(
            view()
                .size(px(STAT_TILE_SIZE))
                .rounded(px(9.0))
                .flex_shrink_0()
                .border_1()
                .border_color(move || {
                    let alpha = if is_active() { 0x66 } else { 0x2e };
                    rgba((hue << 8) | alpha)
                })
                .bg(move || {
                    let alpha = if is_active() { 0x36 } else { 0x1a };
                    rgba((hue << 8) | alpha)
                })
                .items_center()
                .justify_center()
                .child(svg(icon_src).size(px(STAT_ICON_SIZE)).text_color(rgb(hue))),
        )
        // Value + label + sub-metrics
        .child(
            view()
                .flex_1()
                .flex_col()
                .justify_center()
                .gap(px(2.0))
                .overflow_hidden()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(
                            text(count_fn)
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(move || {
                                    if is_active() {
                                        rgb(hue)
                                    } else {
                                        store.theme().text_1
                                    }
                                }),
                        )
                        .child(
                            text(move || store.tr(title_key))
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || store.theme().text_2),
                        ),
                )
                .child(
                    text(detail_fn)
                        .text_size(px(10.5))
                        .text_color(move || store.theme().text_2),
                ),
        )
}

/// Key-value metric pair inside the XERJ status bar.
fn metric_pair(
    store: AppStore,
    label: &'static str,
    value_fn: impl Fn() -> String + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            text(label)
                .text_size(px(10.5))
                .text_color(move || store.theme().text_3),
        )
        .child(
            text(value_fn)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || store.theme().text_1),
        )
}

/// Hairline vertical divider between XERJ metrics.
fn metric_divider(store: AppStore) -> impl IntoElement {
    view()
        .w(px(1.0))
        .h(px(10.0))
        .bg(move || store.theme().divider)
}
