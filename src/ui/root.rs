//! Root view assembling all UI components into the complete Clippi-style GUI.

use nsh_core::prelude::*;

use super::bottom_bar::bottom_bar_view;
use super::clipboard_list::clipboard_list_view;
use super::components::confirm_dialog::confirm_dialog_view;
use super::components::toast::toast_view;
use super::edit_panel::edit_panel_view;
use super::filter_bar::filter_bar_view;
use super::quick_paste::execute_quick_copy;
use super::search_box::search_box_view;
use super::settings::settings_panel_view;
use super::stats_overview::stats_overview_view;
use super::tag_filter::tag_filter_view;
use super::tag_picker::tag_picker_view;
use super::theme::ClippiTheme;
use super::titlebar::titlebar_view;
use super::type_filter_config::type_filter_config_view;

use crate::core::types::KnowledgeItem;
use crate::state::app::{ActiveView, AppStore};

/// Root view of the amemory GUI.
pub fn root_view(
    store: AppStore,
    on_trigger_sync: impl Fn() + Send + Sync + 'static,
    on_check_update: impl Fn() + Send + Sync + 'static,
    on_start_download: impl Fn() + Send + Sync + 'static,
    on_restart_update: impl Fn() + Send + Sync + 'static,
    on_rebuild_index: impl Fn() + Send + Sync + 'static,
    on_save_item: impl Fn(KnowledgeItem, bool) + Send + Sync + 'static,
    on_delete_item: impl Fn(String) + Send + Sync + 'static,
) -> impl IntoElement {
    let on_trigger_sync = std::sync::Arc::new(on_trigger_sync);
    let on_check_update = std::sync::Arc::new(on_check_update);
    let on_start_download = std::sync::Arc::new(on_start_download);
    let on_restart_update = std::sync::Arc::new(on_restart_update);
    let on_rebuild_index = std::sync::Arc::new(on_rebuild_index);
    let on_save_item = std::sync::Arc::new(on_save_item);
    let on_delete_item = std::sync::Arc::new(on_delete_item);

    let active_view = store.active_view;
    let toast = store.toast;
    let confirm_dialog = store.confirm_dialog;
    let show_stats = store.show_stats_overview;
    let show_type_config = signal(false);
    let show_tag_filter = signal(false);
    let tag_picking_item: Signal<Option<KnowledgeItem>> = signal(None);

    let get_theme = move || ClippiTheme::resolve(store.settings.get().theme_mode);

    let theme = get_theme();
    let panel_bg = theme.panel_bg;

    let sync_for_title = on_trigger_sync.clone();
    let delete_for_list = on_delete_item;
    let save_for_edit = on_save_item.clone();
    let save_for_picker = on_save_item;

    view()
        .w_full()
        .h_full()
        .rounded(px(12.0))
        .bg(panel_bg)
        .flex_col()
        .relative()
        // ── 1. Top 30px Custom Titlebar ──
        .child(titlebar_view(store, theme, move || sync_for_title()))
        // ── 2. Main Content Routing ──
        .child(
            view()
                .flex_1()
                .w_full()
                .relative()
                .child(
                    when(
                        move || matches!(active_view.get(), ActiveView::ClipboardList),
                        {
                            let del_cb = delete_for_list;
                            move || {
                                let del_cb2 = del_cb.clone();
                                view()
                                    .w_full()
                                    .h_full()
                                    .flex_col()
                                    // Search Box (28px)
                                    .child(search_box_view(
                                        store,
                                        theme,
                                        move |_query| {},
                                    ))
                                    // Dual-row Filter Bar (22px each row)
                                    .child(filter_bar_view(
                                        store,
                                        theme,
                                        move |_| {},
                                        move |_| {},
                                    ))
                                    // Stats Overview (when toggled on)
                                    .child(
                                        when(
                                            move || show_stats.get(),
                                            move || stats_overview_view(store, theme),
                                        ),
                                    )
                                    // Scrollable Card List
                                    .child(clipboard_list_view(
                                        store,
                                        store.card_height(),
                                        theme,
                                        // on_copy
                                        move |item| {
                                            execute_quick_copy(store, &item);
                                        },
                                        // on_edit
                                        move |item| {
                                            store.open_edit_item(item);
                                        },
                                        // on_toggle_fav
                                        move |item| {
                                            store.items.update(|list| {
                                                if let Some(it) = list.iter_mut().find(|x| x.id == item.id) {
                                                    it.favorite = !it.favorite;
                                                    it.is_favorite = it.favorite;
                                                }
                                            });
                                        },
                                        // on_delete
                                        {
                                            let del_cb3 = del_cb2;
                                            move |item| {
                                                let delete_id = item.id;
                                                let del_run = del_cb3.clone();
                                                store.show_confirm_dialog(
                                                    "删除知识项",
                                                    "确定要彻底删除该知识条目吗？此操作不可逆。",
                                                    move || {
                                                        del_run(delete_id.clone());
                                                    },
                                                );
                                            }
                                        },
                                    ))
                                    // Bottom 26px Status Bar
                                    .child({
                                        let sync_bottom = on_trigger_sync.clone();
                                        bottom_bar_view(
                                            store,
                                            theme,
                                            move || sync_bottom(),
                                            move || {
                                                crate::platform::clipboard::write_text("http://127.0.0.1:8000/mcp");
                                                store.show_toast("已复制 MCP 端点地址");
                                            },
                                        )
                                    })
                            }
                        },
                    ),
                )
                // Edit Panel Router
                .child(
                    when(
                        move || matches!(active_view.get(), ActiveView::Edit),
                        {
                            let save_cb = save_for_edit;
                            move || {
                                let save_cb2 = save_cb.clone();
                                edit_panel_view(
                                    store,
                                    theme,
                                    move |item, is_new| {
                                        save_cb2(item, is_new);
                                        store.active_view.set(ActiveView::ClipboardList);
                                    },
                                    move || {
                                        store.active_view.set(ActiveView::ClipboardList);
                                    },
                                )
                            }
                        },
                    ),
                )
                // Settings Panel Router
                .child(
                    when(
                        move || matches!(active_view.get(), ActiveView::Settings(_)),
                        {
                            let c = on_check_update;
                            let d = on_start_download;
                            let r = on_restart_update;
                            let b = on_rebuild_index;
                            move || {
                                let c2 = c.clone();
                                let d2 = d.clone();
                                let r2 = r.clone();
                                let b2 = b.clone();
                                settings_panel_view(
                                    store,
                                    theme,
                                    move || c2(),
                                    move || d2(),
                                    move || r2(),
                                    move || b2(),
                                )
                            }
                        },
                    ),
                ),
        )
        // ── 3. Overlays & Dialogs ──
        // Tag Filter Modal
        .child(
            when(
                move || show_tag_filter.get(),
                move || {
                    tag_filter_view(
                        store,
                        theme,
                        move || {
                            show_tag_filter.set(false);
                        },
                    )
                },
            ),
        )
        // Type Config Modal
        .child(
            when(
                move || show_type_config.get(),
                move || {
                    type_filter_config_view(
                        store,
                        theme,
                        move || {
                            show_type_config.set(false);
                        },
                    )
                },
            ),
        )
        // Tag Picker Modal
        .child(
            when(
                move || tag_picking_item.get().is_some(),
                {
                    let save_picker = save_for_picker;
                    move || {
                        let item = tag_picking_item.get().unwrap_or_default();
                        let save_picker2 = save_picker.clone();
                        tag_picker_view(
                            store,
                            item,
                            theme,
                            move || {
                                tag_picking_item.set(None);
                            },
                            move |updated| {
                                save_picker2(updated, false);
                            },
                        )
                    }
                },
            ),
        )
        // Confirm Dialog
        .child(
            when(
                move || confirm_dialog.get().is_some(),
                move || {
                    let diag = confirm_dialog.get().unwrap_or_else(|| crate::state::app::ConfirmDialogData {
                        title: String::new(),
                        message: String::new(),
                        on_confirm: std::sync::Arc::new(|| {}),
                    });
                    let confirm_cb = diag.on_confirm.clone();
                    confirm_dialog_view(
                        diag.title,
                        diag.message,
                        theme,
                        move || {
                            confirm_cb();
                            store.dismiss_confirm_dialog();
                        },
                        move || {
                            store.dismiss_confirm_dialog();
                        },
                    )
                },
            ),
        )
        // Floating Toast Banner
        .child(
            when(
                move || toast.get().is_some(),
                move || {
                    let msg = toast.get().unwrap_or_default();
                    toast_view(msg, theme)
                },
            ),
        )
}
