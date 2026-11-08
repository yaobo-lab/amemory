//! Background polling loop for stats refresh and periodic health/update checks.

use std::sync::Arc;
use std::time::Duration;

use toolkit_rs::chrono::Utc;

use super::amemory_runtime::AmemoryRuntime;
use super::update::{UpdateChecker, UpdatePhase, scheduled_update_check_due};
use crate::core::settings::GuiSettings;
use crate::state::app::AppStore;

/// Background loop refreshing stats and checking for updates every 5s / 24h.
pub async fn start_background_poll_loop(runtime: Arc<AmemoryRuntime>, store: AppStore) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut last_update_check = String::new();

    loop {
        interval.tick().await;

        // 1. Refresh live stats snapshot
        let stats = runtime.get_stats_snapshot();
        store.set_stats(stats);

        // 2. Periodic scheduled update check
        let settings = GuiSettings::load();
        if settings.auto_check_updates && scheduled_update_check_due(&last_update_check, Utc::now())
        {
            last_update_check = Utc::now().to_rfc3339();
            let checker = UpdateChecker::new(env!("CARGO_PKG_VERSION"), "xcat", "amemory");
            if let Ok(Some(info)) = checker.check_github().await {
                store.set_update_phase(UpdatePhase::Available(info.clone()));
                store.set_update_available(Some(info));
            }
        }
    }
}
