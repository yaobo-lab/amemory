//! 30px Top Titlebar matching Clippi 1:1.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::assets::{action_icons, glyphs};
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::i18n_keys::I18nKey;
use crate::state::app::{ActiveView, AppStore};

/// Render top 30px custom titlebar.
pub fn titlebar_view(
    store: AppStore,
    _theme: ClippiTheme,
    on_trigger_sync: impl Fn() + 'static,
) -> impl IntoElement {
    let is_pinned = store.is_pinned;
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

    let copy_mcp_endpoint = move || {
        let url = store.mcp_endpoint_url();
        crate::platform::clipboard::write_text(&url);
        store.show_toast(format!("已复制 MCP 地址: {url}"));
    };

    let mcp_icon_src = ImageSrc::svg_markup(action_icons::COPY_MCP);

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
        .bg(move || store.theme().titlebar_bg)
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(10.0))
        .pr(move || {
            let right_sys = nsh_core::element::desktop::title_bar().right.0;
            let default_right = if cfg!(windows) { 140.0 } else { 4.0 };
            px(right_sys.max(default_right))
        })
        // Window drag spacer area on the left (title moved to native window frame)
        .child(view().flex_1().h_full().window_drag(true))
        // Right-side 28x28 action icon buttons
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .window_drag(false)
                // 1. Copy MCP endpoint address
                .child(
                    view()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(store.theme().btn_hover))
                        .active(|s| s.opacity(0.7))
                        .on_press(copy_mcp_endpoint)
                        .tooltip(tooltip_box(
                            store.tr(I18nKey::BottomCopyEndpoint),
                            store.theme(),
                        ))
                        .child(
                            svg(mcp_icon_src)
                                .size(px(15.0))
                                .text_color(move || store.theme().text_2),
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
                        .hover(move |s| s.bg(store.theme().btn_hover))
                        .on_press(on_trigger_sync)
                        .tooltip(tooltip_box(
                            store.tr(I18nKey::TitlebarSyncCatalog),
                            store.theme(),
                        ))
                        .child(
                            text(glyphs::SYNC)
                                .font_family("iconfont")
                                .text_color(move || {
                                    if is_syncing.get() {
                                        store.theme().accent
                                    } else {
                                        store.theme().text_2
                                    }
                                })
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
                        .hover(move |s| s.bg(store.theme().btn_hover))
                        .on_press(toggle_fav)
                        .tooltip(tooltip_box(
                            store.tr(I18nKey::TitlebarFilterFavorite),
                            store.theme(),
                        ))
                        .child(
                            text(glyphs::STAR)
                                .font_family("iconfont")
                                .text_color(move || {
                                    if filters.with(|f| f.favorites_only) {
                                        store.theme().fav_color
                                    } else {
                                        store.theme().text_2
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
                        .hover(move |s| s.bg(store.theme().btn_hover))
                        .on_press(toggle_pin)
                        .tooltip(tooltip_box(
                            store.tr(I18nKey::TitlebarAlwaysOnTop),
                            store.theme(),
                        ))
                        .child(
                            text(glyphs::PIN)
                                .font_family("iconfont")
                                .text_color(move || {
                                    if is_pinned.get() {
                                        store.theme().accent
                                    } else {
                                        store.theme().text_2
                                    }
                                })
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
                        .hover(move |s| s.bg(store.theme().btn_hover))
                        .on_press(open_settings)
                        .tooltip(tooltip_box(
                            store.tr(I18nKey::TitlebarSettings),
                            store.theme(),
                        ))
                        .child(
                            view()
                                .relative()
                                .child(
                                    text(glyphs::SETTINGS)
                                        .font_family("iconfont")
                                        .text_color(move || store.theme().text_2)
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
                                            .bg(move || store.theme().accent)
                                    },
                                )),
                        ),
                ),
        )
}
