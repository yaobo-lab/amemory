//! Root view assembling all UI components into the complete Clippi-style GUI.

use nsh_core::prelude::*;

use super::bottom_bar::bottom_bar_view;
use super::clipboard_list::clipboard_list_view;
use super::components::confirm_dialog::confirm_dialog_view;
use super::components::toast::toast_view;
use super::context_menu::context_menu_modal;
use super::edit_panel::edit_panel_view;
use super::filter_bar::filter_bar_view;
use super::quick_paste::execute_quick_copy;
use super::search_box::search_box_view;
use super::settings::settings_panel_view;
use super::sidebar::sidebar_view;
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
    let show_type_config = store.show_type_config;
    let show_tag_filter = store.show_tag_filter;
    let tag_picking_item = store.tag_picking_item;
    let context_menu_item = store.context_menu_item;
    let is_sidebar_open = store.is_sidebar_open;

    let get_theme = move || {
        let s = store.settings.get();
        ClippiTheme::resolve_with_accent(s.theme_mode, s.accent_color.as_deref())
    };

    let theme = get_theme();

    let sync_for_title = on_trigger_sync.clone();
    let delete_for_list = on_delete_item.clone();
    let delete_for_ctx = on_delete_item;
    let save_for_edit = on_save_item.clone();
    let save_for_picker = on_save_item;

    view()
        .w_full()
        .h_full()
        .rounded(px(12.0))
        .bg(move || get_theme().panel_bg)
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
                                let cur_theme = get_theme();
                                let del_cb2 = del_cb.clone();
                                view()
                                    .w_full()
                                    .h_full()
                                    .flex_col()
                                    // Search Box (28px)
                                    .child(search_box_view(
                                        store,
                                        cur_theme,
                                        move |_query| {},
                                    ))
                                    // Dual-row Filter Bar (22px each row)
                                    .child(filter_bar_view(
                                        store,
                                        cur_theme,
                                        move |_| {},
                                        move |_| {},
                                    ))
                                    // Stats Overview (when toggled on)
                                    .child(
                                        when(
                                            move || show_stats.get(),
                                            move || stats_overview_view(store, get_theme()),
                                        ),
                                    )
                                    // Sidebar + Scrollable Card List
                                    .child(
                                        view()
                                            .flex_1()
                                            .w_full()
                                            .flex_row()
                                            .overflow_hidden()
                                            .child(
                                                when(
                                                    move || is_sidebar_open.get(),
                                                    move || sidebar_view(store, get_theme()),
                                                ),
                                            )
                                            .child(clipboard_list_view(
                                                store,
                                                store.card_height(),
                                                cur_theme,
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
                                                    let fav = store.toggle_item_favorite(&item.id);
                                                    store.show_toast(if fav {
                                                        "已加入收藏"
                                                    } else {
                                                        "已取消收藏"
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
                                            )),
                                    )
                                    // Bottom 26px Status Bar
                                    .child({
                                        let sync_bottom = on_trigger_sync.clone();
                                        bottom_bar_view(
                                            store,
                                            cur_theme,
                                            move || sync_bottom(),
                                            move || {
                                                let url = store.mcp_endpoint_url();
                                                crate::platform::clipboard::write_text(&url);
                                                store.show_toast(format!(
                                                    "已复制 MCP 端点地址: {url}"
                                                ));
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
                                    get_theme(),
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
                                    get_theme(),
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
                        get_theme(),
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
                        get_theme(),
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
                            get_theme(),
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
        // Context Menu Modal
        .child(
            when(
                move || context_menu_item.get().is_some(),
                {
                    let del_ctx = delete_for_ctx;
                    move || {
                        let item = context_menu_item.get().unwrap_or_default();
                        let title = item.title.clone();
                        let item_for_copy = item.clone();
                        let item_for_uri = item.clone();
                        let item_for_tags = item.clone();
                        let item_for_dir = item.clone();
                        let item_for_edit = item.clone();
                        let item_for_fav = item.clone();
                        let item_for_del = item;
                        let del_ctx2 = del_ctx.clone();
                        context_menu_modal(
                            title,
                            get_theme(),
                            // on_copy
                            move || {
                                execute_quick_copy(store, &item_for_copy);
                                context_menu_item.set(None);
                            },
                            // on_copy_uri
                            move || {
                                crate::platform::clipboard::write_text(&item_for_uri.uri);
                                store.show_toast(format!("已复制 URI: {}", item_for_uri.uri));
                                context_menu_item.set(None);
                            },
                            // on_manage_tags
                            move || {
                                context_menu_item.set(None);
                                store.tag_picking_item.set(Some(item_for_tags.clone()));
                            },
                            // on_open_dir
                            move || {
                                context_menu_item.set(None);
                                let cfg = store.app_config.get();
                                let path = if let Some(ref sp) = item_for_dir.source_path {
                                    std::path::PathBuf::from(sp)
                                } else if item_for_dir.kind == crate::core::types::ItemKind::Skill {
                                    cfg.storage.skills.join(&item_for_dir.owner)
                                } else if item_for_dir.kind == crate::core::types::ItemKind::Book {
                                    cfg.storage.books.join(&item_for_dir.owner)
                                } else {
                                    cfg.xerj.data_dir
                                };
                                crate::services::amemory_runtime::open_path_in_explorer(&path);
                            },
                            // on_edit
                            move || {
                                context_menu_item.set(None);
                                store.open_edit_item(item_for_edit.clone());
                            },
                            // on_fav
                            move || {
                                context_menu_item.set(None);
                                let fav = store.toggle_item_favorite(&item_for_fav.id);
                                store.show_toast(if fav {
                                    "已加入收藏"
                                } else {
                                    "已取消收藏"
                                });
                            },
                            // on_delete
                            move || {
                                context_menu_item.set(None);
                                let delete_id = item_for_del.id.clone();
                                let del_run = del_ctx2.clone();
                                store.show_confirm_dialog(
                                    "删除知识项",
                                    "确定要彻底删除该知识条目吗？此操作不可逆。",
                                    move || {
                                        del_run(delete_id.clone());
                                    },
                                );
                            },
                            // on_close
                            move || {
                                context_menu_item.set(None);
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
                    let diag = confirm_dialog.get().unwrap_or_else(|| {
                        crate::state::app::ConfirmDialogData {
                            title: String::new(),
                            message: String::new(),
                            on_confirm: std::sync::Arc::new(|| {}),
                        }
                    });
                    let confirm_cb = diag.on_confirm.clone();
                    confirm_dialog_view(
                        diag.title,
                        diag.message,
                        get_theme(),
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
                    toast_view(msg, get_theme())
                },
            ),
        )
}
