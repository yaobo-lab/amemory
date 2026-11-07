//! GUI Window Manager and application lifecycle bootstrap.

use std::sync::{Arc, Mutex, mpsc};

use nsh_core::element::{Component, IntoElement, NoEvent, Scope, set_interval};
use nsh_platform_windows::{AppOptions, AppSource, Root, run_app};

use super::root::root_view;
use crate::config::AppConfig;
use crate::core::types::KnowledgeItem;
use crate::platform::window_frame::{DEFAULT_WINDOW_H, DEFAULT_WINDOW_W};
use crate::services::amemory_runtime::AmemoryRuntime;
use crate::state::app::AppStore;

type UiAction = Box<dyn FnOnce(AppStore) + Send + 'static>;

/// Global shared state handle for GUI component initialization
static GLOBAL_RUNTIME: std::sync::OnceLock<Arc<AmemoryRuntime>> = std::sync::OnceLock::new();
static GLOBAL_CONFIG: std::sync::OnceLock<AppConfig> = std::sync::OnceLock::new();
static TOKIO_HANDLE: std::sync::OnceLock<tokio::runtime::Handle> = std::sync::OnceLock::new();

fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    if let Some(handle) = TOKIO_HANDLE.get() {
        handle.spawn(future);
    } else {
        tokio::spawn(future);
    }
}

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

        // Channel for background tasks to safely dispatch mutations to the UI engine thread
        let (ui_tx, ui_rx) = mpsc::channel::<UiAction>();
        let ui_rx = Arc::new(Mutex::new(ui_rx));
        let rx_store = store;
        let rx_holder = ui_rx.clone();
        set_interval(std::time::Duration::from_millis(50), move || {
            if let Ok(rx) = rx_holder.try_lock() {
                while let Ok(action) = rx.try_recv() {
                    action(rx_store);
                }
            }
        });

        // 4. Start background polling loop (stats every 5s)
        let rt_for_poll = runtime.clone();
        let poll_tx = ui_tx.clone();
        spawn_task(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let stats = rt_for_poll.get_stats_snapshot();
                let _ = poll_tx.send(Box::new(move |st| {
                    st.set_stats(stats);
                }));
            }
        });

        // 5. Trigger catalog sync handler
        let rt_for_sync = runtime.clone();
        let sync_tx = ui_tx.clone();
        let on_trigger_sync = move || {
            let rt = rt_for_sync.clone();
            let tx = sync_tx.clone();
            let _ = tx.send(Box::new(|st| st.set_syncing(true)));
            spawn_task(async move {
                let reloaded = rt.load_all_knowledge_items();
                let stats = rt.get_stats_snapshot();
                let _ = tx.send(Box::new(move |st| {
                    st.set_items(reloaded);
                    st.set_stats(stats);
                    st.set_syncing(false);
                    st.show_toast("知识库已同步完成");
                }));
            });
        };

        // 6. Update checker handlers
        let check_tx = ui_tx.clone();
        let on_check_update = move || {
            let tx = check_tx.clone();
            let _ = tx.send(Box::new(|st| st.set_update_phase(crate::services::update::UpdatePhase::Checking)));
            spawn_task(async move {
                let checker = crate::services::update::UpdateChecker::new(
                    env!("CARGO_PKG_VERSION"),
                    "xcat",
                    "amemory",
                );
                match checker.check_github().await {
                    Ok(Some(info)) => {
                        let _ = tx.send(Box::new(move |st| {
                            st.set_update_phase(crate::services::update::UpdatePhase::Available(info.clone()));
                            st.set_update_available(Some(info));
                        }));
                    }
                    Ok(None) => {
                        let _ = tx.send(Box::new(|st| {
                            st.set_update_phase(crate::services::update::UpdatePhase::UpToDate);
                        }));
                    }
                    Err(e) => {
                        let _ = tx.send(Box::new(move |st| {
                            st.set_update_phase(crate::services::update::UpdatePhase::Error(e));
                        }));
                    }
                }
            });
        };

        let dl_tx = ui_tx.clone();
        let on_start_download = move || {
            let tx = dl_tx.clone();
            let _ = tx.send(Box::new(|st| {
                st.show_toast("开始下载更新...");
                spawn_task(async move {
                    if let Err(e) = crate::services::updater::AppUpdater::execute_download_and_verify(st).await {
                        st.set_update_phase(crate::services::update::UpdatePhase::Error(e.clone()));
                        st.show_toast(format!("下载更新失败: {e}"));
                    }
                });
            }));
        };

        let restart_tx = ui_tx.clone();
        let on_restart_update = move || {
            let tx = restart_tx.clone();
            let _ = tx.send(Box::new(|st| {
                st.show_toast("准备重启并应用更新...");
                if let Err(e) = crate::services::updater::AppUpdater::apply_and_restart(st) {
                    st.show_toast(format!("启动更新安装器失败: {e}"));
                }
            }));
        };

        // 7. Rebuild search index handler
        let rt_for_rebuild = runtime.clone();
        let rebuild_tx = ui_tx.clone();
        let on_rebuild_index = move || {
            let rt = rt_for_rebuild.clone();
            let tx = rebuild_tx.clone();
            let _ = tx.send(Box::new(|st| st.show_toast("开始全量重建索引...")));
            spawn_task(async move {
                let items = rt.load_all_knowledge_items();
                for item in &items {
                    let _ = rt.index_item(item);
                }
                let stats = rt.get_stats_snapshot();
                let _ = tx.send(Box::new(move |st| {
                    st.set_stats(stats);
                    st.show_toast("索引重建完毕！");
                }));
            });
        };

        // 8. Knowledge item save handler
        let rt_for_save = runtime.clone();
        let save_tx = ui_tx.clone();
        let on_save_item = move |item: KnowledgeItem, is_new: bool| {
            let rt = rt_for_save.clone();
            let tx = save_tx.clone();
            spawn_task(async move {
                let res = rt.save_knowledge_item(&item);
                let _ = tx.send(Box::new(move |st| {
                    match res {
                        Ok(()) => {
                            let reloaded = rt.load_all_knowledge_items();
                            let stats = rt.get_stats_snapshot();
                            st.set_items(reloaded);
                            st.set_stats(stats);
                            st.show_toast(if is_new { "已新建知识项" } else { "已保存知识项" });
                        }
                        Err(e) => {
                            st.show_toast(format!("保存失败: {e}"));
                        }
                    }
                }));
            });
        };

        // 9. Knowledge item delete handler
        let rt_for_delete = runtime;
        let delete_tx = ui_tx;
        let on_delete_item = move |id: String| {
            let rt = rt_for_delete.clone();
            let tx = delete_tx.clone();
            spawn_task(async move {
                let res = rt.delete_knowledge_item(&id);
                let _ = tx.send(Box::new(move |st| {
                    match res {
                        Ok(()) => {
                            let reloaded = rt.load_all_knowledge_items();
                            let stats = rt.get_stats_snapshot();
                            st.set_items(reloaded);
                            st.set_stats(stats);
                            st.show_toast("已删除知识项");
                        }
                        Err(e) => {
                            st.show_toast(format!("删除失败: {e}"));
                        }
                    }
                }));
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
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let _ = TOKIO_HANDLE.set(handle);
    }

    let root = Root::of::<AmemoryGuiComponent>()
        .app_id("com.amemory.app");

    let options = AppOptions {
        app_id: "com.amemory.app".to_string(),
        title: "amemory".to_string(),
        size: (DEFAULT_WINDOW_W as i32, DEFAULT_WINDOW_H as i32),
        ..AppOptions::default()
    };

    let _ = run_app(AppSource::Rust(root), options);
}
