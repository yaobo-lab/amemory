//! GUI Window Manager and application lifecycle bootstrap.

use std::sync::Arc;

use nsh_core::element::{Component, IntoElement, NoEvent, Scope};
use nsh_platform_windows::{run_app, AppOptions, AppSource, Root};

use super::root::root_view;
use crate::config::AppConfig;
use crate::core::types::KnowledgeItem;
use crate::platform::window_frame::{DEFAULT_WINDOW_H, DEFAULT_WINDOW_W};
use crate::services::amemory_runtime::AmemoryRuntime;
use crate::services::poll_loop::start_background_poll_loop;
use crate::services::updater::AppUpdater;
use crate::state::app::AppStore;

/// Global shared state handle for GUI component initialization
static GLOBAL_RUNTIME: std::sync::OnceLock<Arc<AmemoryRuntime>> = std::sync::OnceLock::new();
static GLOBAL_CONFIG: std::sync::OnceLock<AppConfig> = std::sync::OnceLock::new();

/// Main GUI Root Component implementing `nsh_core::element::Component`.
pub struct AmemoryGuiComponent;

impl Component for AmemoryGuiComponent {
    type Props = ();
    type Event = NoEvent;

    fn view(_props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
        let runtime = GLOBAL_RUNTIME
            .get()
            .expect("GLOBAL_RUNTIME must be initialized")
            .clone();
        let _config = GLOBAL_CONFIG
            .get()
            .expect("GLOBAL_CONFIG must be initialized");

        // 1. Create reactive application store
        let store = AppStore::new();

        // 2. Initial load of knowledge items from runtime
        let initial_items = runtime.load_all_knowledge_items();
        store.set_items(initial_items);

        // 3. Initial stats load
        let initial_stats = runtime.get_stats_snapshot();
        store.set_stats(initial_stats);

        // 4. Start background polling loop (stats every 5s, updates every 24h)
        let rt_for_poll = runtime.clone();
        let store_for_poll = store.clone();
        tokio::spawn(async move {
            start_background_poll_loop(rt_for_poll, store_for_poll).await;
        });

        // 5. Trigger catalog sync handler
        let rt_for_sync = runtime.clone();
        let store_for_sync = store.clone();
        let on_trigger_sync = move || {
            let rt = rt_for_sync.clone();
            let st = store_for_sync.clone();
            st.set_syncing(true);
            tokio::spawn(async move {
                let reloaded = rt.load_all_knowledge_items();
                let stats = rt.get_stats_snapshot();
                st.set_items(reloaded);
                st.set_stats(stats);
                st.set_syncing(false);
                st.show_toast("知识库已同步完成");
            });
        };

        // 6. Update checker handlers
        let store_for_check = store.clone();
        let on_check_update = move || {
            let st = store_for_check.clone();
            tokio::spawn(async move {
                AppUpdater::check_and_update_state(st).await;
            });
        };

        let store_for_dl = store.clone();
        let on_start_download = move || {
            let st = store_for_dl.clone();
            tokio::spawn(async move {
                let res = AppUpdater::execute_download_and_verify(st.clone()).await;
                if let Err(e) = res {
                    st.set_update_phase(crate::services::update::UpdatePhase::Error(e.to_string()));
                }
            });
        };

        let store_for_restart = store.clone();
        let on_restart_update = move || {
            let st = store_for_restart.clone();
            if let Err(e) = AppUpdater::apply_and_restart(st.clone()) {
                st.show_toast(format!("升级重启失败: {}", e));
            }
        };

        // 7. Rebuild search index handler
        let rt_for_rebuild = runtime.clone();
        let store_for_rebuild = store.clone();
        let on_rebuild_index = move || {
            let rt = rt_for_rebuild.clone();
            let st = store_for_rebuild.clone();
            tokio::spawn(async move {
                st.show_toast("开始全量重建索引...");
                let items = rt.load_all_knowledge_items();
                for item in &items {
                    let _ = rt.index_item(item);
                }
                let stats = rt.get_stats_snapshot();
                st.set_stats(stats);
                st.show_toast("索引重建完毕！");
            });
        };

        // 8. Knowledge item save handler
        let rt_for_save = runtime.clone();
        let store_for_save = store.clone();
        let on_save_item = move |item: KnowledgeItem, is_new: bool| {
            let rt = rt_for_save.clone();
            let st = store_for_save.clone();
            tokio::spawn(async move {
                let res = rt.save_knowledge_item(&item);
                match res {
                    Ok(_) => {
                        let reloaded = rt.load_all_knowledge_items();
                        let stats = rt.get_stats_snapshot();
                        st.set_items(reloaded);
                        st.set_stats(stats);
                        st.show_toast(if is_new { "已新建知识项" } else { "已保存知识项" });
                    }
                    Err(e) => {
                        st.show_toast(format!("保存失败: {}", e));
                    }
                }
            });
        };

        // 9. Knowledge item delete handler
        let rt_for_delete = runtime.clone();
        let store_for_delete = store.clone();
        let on_delete_item = move |id: String| {
            let rt = rt_for_delete.clone();
            let st = store_for_delete.clone();
            tokio::spawn(async move {
                let res = rt.delete_knowledge_item(&id);
                match res {
                    Ok(_) => {
                        let reloaded = rt.load_all_knowledge_items();
                        let stats = rt.get_stats_snapshot();
                        st.set_items(reloaded);
                        st.set_stats(stats);
                        st.show_toast("已删除知识项");
                    }
                    Err(e) => {
                        st.show_toast(format!("删除失败: {}", e));
                    }
                }
            });
        };

        // Assemble root view
        root_view(
            store,
            on_trigger_sync,
            on_check_update,
            on_start_download,
            on_restart_update,
            on_rebuild_index,
            on_save_item,
            on_delete_item,
        )
    }
}

/// Launch the complete Native Shell GUI application.
pub fn run_gui(runtime: Arc<AmemoryRuntime>, config: AppConfig) {
    let _ = GLOBAL_RUNTIME.set(runtime);
    let _ = GLOBAL_CONFIG.set(config);

    let root = Root::of::<AmemoryGuiComponent>()
        .app_id("com.amemory.app");

    let mut options = AppOptions::default();
    options.app_id = "com.amemory.app".to_string();
    options.title = "amemory".to_string();
    options.size = (DEFAULT_WINDOW_W as i32, DEFAULT_WINDOW_H as i32);

    let _ = run_app(AppSource::Rust(root), options);
}
