//! Settings panel module with 6 tabs matching Clippi 1:1.

use nsh_core::element::AnyElement;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::{ActiveView, AppStore};

pub mod general;
pub mod storage;
pub mod xerj_engine;
pub mod mcp_hotkey;
pub mod data_logger;
pub mod version;

pub const SETTINGS_TAB_COUNT: usize = 6;

/// Reusable settings group container matching Clippi 1:1.
pub fn settings_group(
    title: &'static str,
    theme: ClippiTheme,
    children: Vec<AnyElement>,
) -> impl IntoElement {
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;

    let mut group = view()
        .w_full()
        .bg(card_bg)
        .rounded(px(8.0))
        .border(px(1.0))
        .border_color(divider)
        .p(px(10.0))
        .flex_col()
        .gap(px(8.0))
        .child(
            text(title)
                .text_color(text_1)
                .font_weight(FontWeight::BOLD)
                .text_xs(),
        );

    for child in children {
        group = group.child(child);
    }
    group
}

/// Helper for standard setting item row (label + description on left, control on right).
pub fn setting_row(
    label: &'static str,
    description: Option<&'static str>,
    control: impl IntoElement + 'static,
    theme: ClippiTheme,
) -> AnyElement {
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;

    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(4.0))
        .child(
            view()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(label)
                        .text_color(text_1)
                        .text_xs(),
                )
                .child(
                    when(
                        move || description.is_some(),
                        move || {
                            text(description.unwrap_or(""))
                                .text_color(text_3)
                                .text_xs()
                        },
                    ),
                ),
        )
        .child(control)
        .into_element()
}

/// Render the complete 6-Tab Settings Panel.
pub fn settings_panel_view(
    store: AppStore,
    theme: ClippiTheme,
    on_check_update: impl Fn() + Send + Sync + 'static,
    on_start_download: impl Fn() + Send + Sync + 'static,
    on_restart_update: impl Fn() + Send + Sync + 'static,
    on_rebuild_index: impl Fn() + Send + Sync + 'static,
) -> impl IntoElement {
    let on_check_update = std::sync::Arc::new(on_check_update);
    let on_start_download = std::sync::Arc::new(on_start_download);
    let on_restart_update = std::sync::Arc::new(on_restart_update);
    let on_rebuild_index = std::sync::Arc::new(on_rebuild_index);

    let active_view = store.active_view;
    let active_tab = signal(match active_view.get() {
        ActiveView::Settings(t) => t,
        _ => 0,
    });

    let tab_titles = [
        I18nKey::SettingTabGeneral.text(),
        I18nKey::SettingTabStorage.text(),
        I18nKey::SettingTabXerj.text(),
        I18nKey::SettingTabMcp.text(),
        I18nKey::SettingTabData.text(),
        I18nKey::SettingTabVersion.text(),
    ];

    let accent = theme.accent;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;

    view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(theme.panel_bg)
        // ── 1. Top 36px Title Bar with Back Button ──
        .child(
            view()
                .h(px(36.0))
                .w_full()
                .bg(card_bg)
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .border_b(px(1.0))
                .border_color(divider)
                .child(
                    view()
                        .cursor_pointer()
                        .on_press(move || {
                            store.active_view.set(ActiveView::ClipboardList);
                        })
                        .child(
                            text(glyphs::BACK)
                                .font_family("iconfont")
                                .text_color(accent)
                                .text_sm(),
                        ),
                )
                .child(
                    text(I18nKey::SettingHeaderTitle.text())
                        .text_color(text_1)
                        .font_weight(FontWeight::BOLD)
                        .text_xs(),
                ),
        )
        // ── 2. 6-Tab Navigation Bar ──
        .child(
            view()
                .h(px(32.0))
                .w_full()
                .bg(card_bg)
                .flex_row()
                .items_center()
                .border_b(px(1.0))
                .border_color(divider)
                .child(
                    tab_button(0, tab_titles[0], active_tab, accent, text_3, text_1),
                )
                .child(
                    tab_button(1, tab_titles[1], active_tab, accent, text_3, text_1),
                )
                .child(
                    tab_button(2, tab_titles[2], active_tab, accent, text_3, text_1),
                )
                .child(
                    tab_button(3, tab_titles[3], active_tab, accent, text_3, text_1),
                )
                .child(
                    tab_button(4, tab_titles[4], active_tab, accent, text_3, text_1),
                )
                .child(
                    tab_button(5, tab_titles[5], active_tab, accent, text_3, text_1),
                ),
        )
        // ── 3. Content Router ──
        .child(
            view()
                .flex_1()
                .w_full()
                .p(px(8.0))
                .child(
                    scroll()
                        .w_full()
                        .h_full()
                        .child(
                            view()
                                .w_full()
                                .flex_col()
                                .gap(px(10.0))
                                .child(
                                    when(
                                        move || active_tab.get() == 0,
                                        move || general::render_general_tab(store, theme),
                                    ),
                                )
                                .child(
                                    when(
                                        move || active_tab.get() == 1,
                                        move || storage::render_storage_tab(store, theme),
                                    ),
                                )
                                .child(
                                    when(
                                        move || active_tab.get() == 2,
                                        {
                                            let on_rebuild = on_rebuild_index.clone();
                                            move || xerj_engine::render_xerj_tab(store, theme, {
                                                let f = on_rebuild.clone();
                                                move || f()
                                            })
                                        },
                                    ),
                                )
                                .child(
                                    when(
                                        move || active_tab.get() == 3,
                                        move || mcp_hotkey::render_mcp_hotkey_tab(store, theme),
                                    ),
                                )
                                .child(
                                    when(
                                        move || active_tab.get() == 4,
                                        move || data_logger::render_data_logger_tab(store, theme),
                                    ),
                                )
                                .child(
                                    when(
                                        move || active_tab.get() == 5,
                                        {
                                            let check = on_check_update.clone();
                                            let dl = on_start_download.clone();
                                            let restart = on_restart_update.clone();
                                            move || {
                                                let c = check.clone();
                                                let d = dl.clone();
                                                let r = restart.clone();
                                                version::render_version_tab(
                                                    store,
                                                    theme,
                                                    move || c(),
                                                    move || d(),
                                                    move || r(),
                                                )
                                            }
                                        },
                                    ),
                                ),
                        ),
                ),
        )
}

fn tab_button(
    index: usize,
    title: &'static str,
    active_tab: Signal<usize>,
    accent: nsh_core::prelude::Rgba,
    text_muted: nsh_core::prelude::Rgba,
    text_active: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .flex_1()
        .h_full()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .border_b(move || if active_tab.get() == index { px(2.0) } else { px(0.0) })
        .border_color(accent)
        .on_press(move || {
            active_tab.set(index);
        })
        .child(
            when(
                move || active_tab.get() == index,
                move || {
                    text(title)
                        .text_color(text_active)
                        .font_weight(FontWeight::BOLD)
                        .text_xs()
                },
            )
            .otherwise(move || {
                text(title)
                    .text_color(text_muted)
                    .font_weight(FontWeight::NORMAL)
                    .text_xs()
            }),
        )
}
