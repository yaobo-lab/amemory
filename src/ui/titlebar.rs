//! 30px Top Titlebar matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::{ActiveView, AppStore};

/// Render top 30px custom titlebar.
pub fn titlebar_view(
    store: AppStore,
    theme: ClippiTheme,
    on_trigger_sync: impl Fn() + 'static,
) -> impl IntoElement {
    let titlebar_bg = theme.titlebar_bg;
    let text_2 = theme.text_2;
    let accent = theme.accent;
    let btn_hover = theme.btn_hover;

    let is_pinned = store.is_pinned;
    let show_stats = store.show_stats_overview;
    let update_phase = store.update_phase;
    let is_syncing = store.is_syncing;
    let filters = store.filters;

    let toggle_pin = move || {
        let next = !is_pinned.get_untracked();
        is_pinned.set(next);
        crate::platform::window_frame::set_always_on_top(next);
        store.show_toast(if next {
            "已开启窗口置顶"
        } else {
            "已取消窗口置顶"
        });
    };

    let toggle_stats = move || {
        let next = !show_stats.get_untracked();
        show_stats.set(next);
        store.update_settings(|s| s.show_stats_overview = next);
    };

    let toggle_fav = move || {
        let mut next = false;
        filters.update(|f| {
            f.favorites_only = !f.favorites_only;
            next = f.favorites_only;
        });
        store.show_toast(if next {
            "已筛选：仅看收藏"
        } else {
            "已显示全部条目"
        });
    };

    let open_settings = move || {
        if matches!(store.active_view.get_untracked(), ActiveView::Settings(_)) {
            store.active_view.set(ActiveView::ClipboardList);
        } else {
            store.active_view.set(ActiveView::Settings(0));
        }
    };

    view()
        .h(px(30.0))
        .w_full()
        .bg(titlebar_bg)
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(10.0))
        .pr(px(4.0))
        // Window drag spacer area on the left (title moved to native window frame)
        .child(view().flex_1().h_full().window_drag(true))
        // Right-side 28x28 action icon buttons
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .window_drag(false)
                // 1. Stats toggle
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(toggle_stats)
                        .tooltip(tooltip_box(I18nKey::TitlebarToggleStats.text(), theme))
                        .child(
                            text(glyphs::STATS)
                                .font_family("iconfont")
                                .text_color(move || if show_stats.get() { accent } else { text_2 })
                                .text_sm(),
                        ),
                )
                // 2. Sync catalog
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(on_trigger_sync)
                        .tooltip(tooltip_box(I18nKey::TitlebarSyncCatalog.text(), theme))
                        .child(
                            text(glyphs::SYNC)
                                .font_family("iconfont")
                                .text_color(move || if is_syncing.get() { accent } else { text_2 })
                                .text_sm(),
                        ),
                )
                // 3. Favorites filter
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(toggle_fav)
                        .tooltip(tooltip_box(I18nKey::TitlebarFilterFavorite.text(), theme))
                        .child(
                            text(glyphs::STAR)
                                .font_family("iconfont")
                                .text_color(move || {
                                    if filters.with(|f| f.favorites_only) {
                                        theme.fav_color
                                    } else {
                                        text_2
                                    }
                                })
                                .text_sm(),
                        ),
                )
                // 4. Always on top pin
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(toggle_pin)
                        .tooltip(tooltip_box(I18nKey::TitlebarAlwaysOnTop.text(), theme))
                        .child(
                            text(glyphs::PIN)
                                .font_family("iconfont")
                                .text_color(move || if is_pinned.get() { accent } else { text_2 })
                                .text_sm(),
                        ),
                )
                // 5. Settings button (with update indicator dot)
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(btn_hover))
                        .on_press(open_settings)
                        .tooltip(tooltip_box(I18nKey::TitlebarSettings.text(), theme))
                        .child(
                            view()
                                .relative()
                                .child(
                                    text(glyphs::SETTINGS)
                                        .font_family("iconfont")
                                        .text_color(text_2)
                                        .text_sm(),
                                )
                                .child(when(
                                    move || {
                                        matches!(
                                            update_phase.get(),
                                            crate::services::update::UpdatePhase::UpdateAvailable
                                                | crate::services::update::UpdatePhase::Available(
                                                    _
                                                )
                                        )
                                    },
                                    move || {
                                        view()
                                            .absolute()
                                            .top(px(-1.0))
                                            .right(px(-1.0))
                                            .size(px(6.0))
                                            .rounded(px(3.0))
                                            .bg(accent)
                                    },
                                )),
                        ),
                ),
        )
}
