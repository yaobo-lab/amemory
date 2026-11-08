//! Settings panel module with 6 tabs matching Clippi 1:1.

use nsh_core::element::AnyElement;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::{ActiveView, AppStore};

pub mod data_logger;
pub mod general;
pub mod mcp_hotkey;
pub mod storage;
pub mod version;
pub mod xerj_engine;

/// Total number of tabs in the settings panel.
pub const SETTINGS_TAB_COUNT: usize = 6;

/// Reusable settings group container matching Clippi 1:1.
#[must_use]
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

/// Reusable settings group container matching Clippi 1:1, dynamically styled from store.
#[must_use]
pub fn settings_group_reactive(
    title: impl Fn() -> String + Copy + 'static,
    store: AppStore,
    children: Vec<AnyElement>,
) -> impl IntoElement {
    let mut group = view()
        .w_full()
        .bg(move || store.theme().card_bg)
        .rounded(px(8.0))
        .border(px(1.0))
        .border_color(move || store.theme().divider)
        .p(px(10.0))
        .flex_col()
        .gap(px(8.0))
        .child(
            text(title)
                .text_color(move || store.theme().text_1)
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
                .child(text(label).text_color(text_1).text_xs())
                .child(when(
                    move || description.is_some(),
                    move || text(description.unwrap_or("")).text_color(text_3).text_xs(),
                )),
        )
        .child(control)
        .into_element()
}

/// Helper for reactive setting item row with dynamic label and description closures.
pub fn setting_row_tr(
    label: impl Fn() -> String + Copy + 'static,
    description: Option<impl Fn() -> String + Copy + 'static>,
    control: impl IntoElement + 'static,
    store: AppStore,
) -> AnyElement {
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
                        .text_color(move || store.theme().text_1)
                        .text_xs(),
                )
                .child(when(
                    move || description.is_some(),
                    move || {
                        text(move || description.map(|d| d()).unwrap_or_default())
                            .text_color(move || store.theme().text_3)
                            .text_xs()
                    },
                )),
        )
        .child(control)
        .into_element()
}

/// Render the complete 6-Tab Settings Panel.
pub fn settings_panel_view(
    store: AppStore,
    _theme: ClippiTheme,
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

    let tab_keys = [
        I18nKey::SettingTabGeneral,
        I18nKey::SettingTabStorage,
        I18nKey::SettingTabXerj,
        I18nKey::SettingTabMcp,
        I18nKey::SettingTabData,
        I18nKey::SettingTabVersion,
    ];

    view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || store.theme().panel_bg)
        // ── 1. Top 36px Title Bar with Back Button ──
        .child(
            view()
                .h(px(36.0))
                .w_full()
                .bg(move || store.theme().card_bg)
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .pl(px(10.0))
                .pr(move || {
                    let right_sys = nsh_core::element::desktop::title_bar().right.0;
                    let default_right = if cfg!(windows) { 140.0 } else { 10.0 };
                    px(right_sys.max(default_right))
                })
                .border_b(px(1.0))
                .border_color(move || store.theme().divider)
                .child(
                    view()
                        .cursor_pointer()
                        .on_press(move || {
                            store.active_view.set(ActiveView::ClipboardList);
                        })
                        .tooltip(tooltip_box(store.tr(I18nKey::BackToList), store.theme()))
                        .child(
                            text(glyphs::BACK)
                                .font_family("iconfont")
                                .text_color(move || store.theme().accent)
                                .text_sm(),
                        ),
                )
                .child(
                    text(move || store.tr(I18nKey::SettingHeaderTitle))
                        .text_color(move || store.theme().text_1)
                        .font_weight(FontWeight::BOLD)
                        .text_xs(),
                )
                .child(view().flex_1().h_full().window_drag(true)),
        )
        // ── 2. 6-Tab Navigation Bar ──
        .child(
            view()
                .h(px(32.0))
                .w_full()
                .bg(move || store.theme().card_bg)
                .flex_row()
                .items_center()
                .border_b(px(1.0))
                .border_color(move || store.theme().divider)
                .child(tab_button(0, tab_keys[0], active_tab, store))
                .child(tab_button(1, tab_keys[1], active_tab, store))
                .child(tab_button(2, tab_keys[2], active_tab, store))
                .child(tab_button(3, tab_keys[3], active_tab, store))
                .child(tab_button(4, tab_keys[4], active_tab, store))
                .child(tab_button(5, tab_keys[5], active_tab, store)),
        )
        // ── 3. Content Router ──
        .child(
            view().flex_1().w_full().p(px(8.0)).child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(10.0))
                        .child(when(
                            move || active_tab.get() == 0,
                            move || general::render_general_tab(store, store.theme()),
                        ))
                        .child(when(
                            move || active_tab.get() == 1,
                            move || storage::render_storage_tab(store, store.theme()),
                        ))
                        .child(when(move || active_tab.get() == 2, {
                            let on_rebuild = on_rebuild_index;
                            move || {
                                let f = on_rebuild.clone();
                                xerj_engine::render_xerj_tab(store, store.theme(), move || f())
                            }
                        }))
                        .child(when(
                            move || active_tab.get() == 3,
                            move || mcp_hotkey::render_mcp_hotkey_tab(store, store.theme()),
                        ))
                        .child(when(
                            move || active_tab.get() == 4,
                            move || data_logger::render_data_logger_tab(store, store.theme()),
                        ))
                        .child(when(move || active_tab.get() == 5, {
                            let check = on_check_update;
                            let dl = on_start_download;
                            let restart = on_restart_update;
                            move || {
                                let c = check.clone();
                                let d = dl.clone();
                                let r = restart.clone();
                                version::render_version_tab(
                                    store,
                                    store.theme(),
                                    move || c(),
                                    move || d(),
                                    move || r(),
                                )
                            }
                        })),
                ),
            ),
        )
}

fn tab_button(
    index: usize,
    key: I18nKey,
    active_tab: Signal<usize>,
    store: AppStore,
) -> impl IntoElement {
    view()
        .flex_1()
        .h_full()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .border_b(move || {
            if active_tab.get() == index {
                px(2.0)
            } else {
                px(0.0)
            }
        })
        .border_color(move || store.theme().accent)
        .on_press(move || {
            active_tab.set(index);
            store.active_view.set(ActiveView::Settings(index));
        })
        .child(
            text(move || store.tr(key))
                .text_color(move || {
                    if active_tab.get() == index {
                        store.theme().text_1
                    } else {
                        store.theme().text_3
                    }
                })
                .font_weight(FontWeight::BOLD)
                .text_xs(),
        )
}
