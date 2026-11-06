//! 30px Top Titlebar matching Clippi 1:1.

use nsh_core::prelude::*;

use super::assets::glyphs;
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
    let text_1 = theme.text_1;
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
    };

    let toggle_stats = move || {
        show_stats.update(|v| *v = !*v);
    };

    let toggle_fav = move || {
        filters.update(|f| f.favorites_only = !f.favorites_only);
    };

    let open_settings = move || {
        store.active_view.set(ActiveView::Settings(0));
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
        // Window drag area on the title / logo section
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .flex_1()
                .window_drag(true)
                .child(
                    text("🧠")
                        .text_sm(),
                )
                .child(
                    text(I18nKey::AppName.text())
                        .text_color(text_1)
                        .font_weight(FontWeight::BOLD)
                        .text_xs(),
                ),
        )
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
                        .child(
                            text(glyphs::STATS)
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
                        .child(
                            text(glyphs::SYNC)
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
                        .child(
                            text(glyphs::STAR)
                                .text_color(move || if filters.with(|f| f.favorites_only) { theme.fav_color } else { text_2 })
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
                        .child(
                            text(glyphs::PIN)
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
                        .child(
                            view()
                                .relative()
                                .child(
                                    text(glyphs::SETTINGS)
                                        .text_color(text_2)
                                        .text_sm(),
                                )
                                .child(
                                    when(
                                        move || matches!(update_phase.get(), crate::services::update::UpdatePhase::UpdateAvailable),
                                        move || {
                                            view()
                                                .absolute()
                                                .top(px(-1.0))
                                                .right(px(-1.0))
                                                .size(px(6.0))
                                                .rounded(px(3.0))
                                                .bg(accent)
                                        },
                                    ),
                                ),
                        ),
                ),
        )
}
