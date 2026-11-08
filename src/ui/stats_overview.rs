//! Home dashboard statistics and XERJ engine metric banner.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::{HealthLevel, ItemKind};
use crate::state::app::AppStore;

/// Render home stats overview cards and XERJ metric bar.
#[must_use]
pub fn stats_overview_view(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;

    let stats = store.stats;
    let filters = store.filters;

    let filter_skills = move || {
        filters.update(|f| {
            f.kind = if f.kind == Some(ItemKind::Skill) {
                None
            } else {
                Some(ItemKind::Skill)
            };
        });
    };
    let filter_books = move || {
        filters.update(|f| {
            f.kind = if f.kind == Some(ItemKind::Book) {
                None
            } else {
                Some(ItemKind::Book)
            };
        });
    };
    let filter_memory = move || {
        filters.update(|f| {
            f.kind = if f.kind == Some(ItemKind::Memory) {
                None
            } else {
                Some(ItemKind::Memory)
            };
        });
    };

    view()
        .w_full()
        .px(px(8.0))
        .mb(px(6.0))
        .gap(px(4.0))
        // 1. Three-card asset grid
        .child(
            view()
                .flex_row()
                .gap(px(6.0))
                // Skills card
                .child(
                    view()
                        .flex_1()
                        .h(px(44.0))
                        .rounded(px(8.0))
                        .bg(move || {
                            if filters.get().kind == Some(ItemKind::Skill) {
                                accent_soft
                            } else {
                                surface
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if filters.get().kind == Some(ItemKind::Skill) {
                                accent
                            } else {
                                divider
                            }
                        })
                        .p(px(6.0))
                        .cursor_pointer()
                        .active(|s| s.opacity(0.7))
                        .on_press(filter_skills)
                        .tooltip(tooltip_box("点击筛选全部技能 (Skills)", theme))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    text(glyphs::SKILL)
                                        .font_family("iconfont")
                                        .text_color(accent)
                                        .text_sm(),
                                )
                                .child(
                                    text(move || {
                                        format!(
                                            "{} {}",
                                            stats.with(|s| s.skills_count),
                                            I18nKey::StatsSkills.text()
                                        )
                                    })
                                    .text_color(text_1)
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                                ),
                        )
                        .child(
                            text(move || {
                                format!(
                                    "{} · {}",
                                    stats.with(|s| format!("{}分片", s.skills_docs)),
                                    stats.with(|s| format!("{}文件", s.skill_files_count))
                                )
                            })
                            .text_color(text_3)
                            .text_xs(),
                        ),
                )
                // Books card
                .child(
                    view()
                        .flex_1()
                        .h(px(44.0))
                        .rounded(px(8.0))
                        .bg(move || {
                            if filters.get().kind == Some(ItemKind::Book) {
                                accent_soft
                            } else {
                                surface
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if filters.get().kind == Some(ItemKind::Book) {
                                accent
                            } else {
                                divider
                            }
                        })
                        .p(px(6.0))
                        .cursor_pointer()
                        .active(|s| s.opacity(0.7))
                        .on_press(filter_books)
                        .tooltip(tooltip_box("点击筛选全部书本 (Books)", theme))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    text(glyphs::BOOK)
                                        .font_family("iconfont")
                                        .text_color(accent)
                                        .text_sm(),
                                )
                                .child(
                                    text(move || {
                                        format!(
                                            "{} {}",
                                            stats.with(|s| s.books_count),
                                            I18nKey::StatsBooks.text()
                                        )
                                    })
                                    .text_color(text_1)
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                                ),
                        )
                        .child(
                            text(move || {
                                format!(
                                    "{} · {}",
                                    stats.with(|s| format!("{}分片", s.books_docs)),
                                    stats.with(|s| format!("{}导入", s.imported_docs_count))
                                )
                            })
                            .text_color(text_3)
                            .text_xs(),
                        ),
                )
                // Memory card
                .child(
                    view()
                        .flex_1()
                        .h(px(44.0))
                        .rounded(px(8.0))
                        .bg(move || {
                            if filters.get().kind == Some(ItemKind::Memory) {
                                accent_soft
                            } else {
                                surface
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if filters.get().kind == Some(ItemKind::Memory) {
                                accent
                            } else {
                                divider
                            }
                        })
                        .p(px(6.0))
                        .cursor_pointer()
                        .active(|s| s.opacity(0.7))
                        .on_press(filter_memory)
                        .tooltip(tooltip_box("点击筛选全部记忆 (Memory)", theme))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    text(glyphs::MEMORY)
                                        .font_family("iconfont")
                                        .text_color(accent)
                                        .text_sm(),
                                )
                                .child(
                                    text(move || {
                                        format!(
                                            "{} {}",
                                            stats.with(|s| s.memory_docs),
                                            I18nKey::StatsMemory.text()
                                        )
                                    })
                                    .text_color(text_1)
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                                ),
                        )
                        .child(
                            text(move || format!("{} 空间", stats.with(|s| s.namespace_count)))
                                .text_color(text_3)
                                .text_xs(),
                        ),
                ),
        )
        // 2. XERJ engine metric bar (24px)
        .child(
            view()
                .h(px(24.0))
                .w_full()
                .rounded(px(6.0))
                .bg(surface)
                .border_1()
                .border_color(divider)
                .px(px(8.0))
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        // Health dot
                        .child(view().size(px(6.0)).rounded(px(3.0)).bg(move || {
                            match stats.with(|s| s.health) {
                                HealthLevel::Green => accent,
                                HealthLevel::Yellow => theme.fav_color,
                                HealthLevel::Red => theme.danger,
                                HealthLevel::Unknown => text_3,
                            }
                        }))
                        .child(
                            text("XERJ")
                                .text_color(text_1)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        )
                        .child(
                            text(move || {
                                format!(
                                    "Docs: {} · Segs: {} · 磁盘: {}",
                                    stats.with(|s| s.total_docs),
                                    stats.with(|s| s.index_count),
                                    stats.with(
                                        super::super::core::types::StatsSnapshot::disk_usage_label
                                    )
                                )
                            })
                            .text_color(text_2)
                            .text_xs(),
                        ),
                )
                .child(
                    text(move || {
                        stats.with(|s| s.last_search_latency_ms).map_or_else(
                            || stats.with(|s| s.embed_mode.clone()),
                            |ms| format!("{ms} ms"),
                        )
                    })
                    .text_color(accent)
                    .text_xs(),
                ),
        )
}
