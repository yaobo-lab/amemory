//! Background polling loop for stats refresh and periodic health/update checks.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;

use super::amemory_runtime::AmemoryRuntime;
use super::update::{UpdateChecker, UpdatePhase, scheduled_update_check_due};
use crate::core::settings::GuiSettings;
use crate::core::types::StatsSnapshot;

/// Background loop refreshing stats and checking for updates.
pub fn start_poll_loop(
    runtime: Arc<AmemoryRuntime>,
    stats_tx: watch::Sender<StatsSnapshot>,
    update_phase_tx: watch::Sender<UpdatePhase>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut last_update_check = String::new();

        loop {
            interval.tick().await;

            // 1. Refresh live stats snapshot
            let stats = runtime.stats_snapshot().await;
            let _ = stats_tx.send(stats);

            // 2. Periodic scheduled update check
            let settings = GuiSettings::load();
            if settings.auto_check_updates
                && scheduled_update_check_due(&last_update_check, Utc::now())
            {
                last_update_check = Utc::now().to_rfc3339();
                let checker = UpdateChecker::new(
                    env!("CARGO_PKG_VERSION"),
                    "xcat",
                    "amemory",
                );
                if let Ok(Some(_info)) = checker.check_github().await {
                    let _ = update_phase_tx.send(UpdatePhase::UpdateAvailable);
                }
            }
        }
    });
}
