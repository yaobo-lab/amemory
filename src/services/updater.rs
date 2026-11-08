//! Update orchestration coordinating download, verification, and installation.

use super::downloader::{download_file, verify_sha256};
use super::install::{cleanup_temp, launch_prepared_update, update_temp_dir};
use super::update::{UpdateChecker, UpdateInfo, UpdatePhase, is_safe_asset_name};
use crate::state::app::AppStore;

/// Download, verify and prepare an update package.
pub async fn download_and_prepare_update(
    info: &UpdateInfo,
    mut phase_callback: impl FnMut(UpdatePhase),
) -> Result<(), String> {
    cleanup_temp();
    let temp_dir = update_temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);

    if !is_safe_asset_name(&info.asset_name) {
        return Err(format!("Invalid asset name: {}", info.asset_name));
    }

    let dest_path = temp_dir.join(&info.asset_name);

    // 1. Download
    phase_callback(UpdatePhase::Downloading { progress: 0.0 });
    download_file(
        &info.download_url,
        &dest_path,
        info.asset_size,
        |progress| {
            phase_callback(UpdatePhase::Downloading {
                progress: f32::from(progress) / 100.0,
            });
        },
    )
    .await?;

    // 2. Verify
    phase_callback(UpdatePhase::Verifying);
    if let Some(expected_sha256) = &info.sha256 {
        verify_sha256(&dest_path, expected_sha256)?;
    }

    // 3. Ready
    phase_callback(UpdatePhase::ReadyToRestart);
    Ok(())
}

/// Restart the app and run the installer.
pub fn apply_update_and_restart(asset_name: &str) -> Result<(), String> {
    let dest_path = update_temp_dir().join(asset_name);
    launch_prepared_update(&dest_path)
}

/// Helper coordinator for `AppStore` update actions.
pub struct AppUpdater;

impl AppUpdater {
    /// Asynchronously check GitHub releases and update the store's phase.
    pub async fn check_and_update_state(store: AppStore) {
        store.set_update_phase(UpdatePhase::Checking);
        let checker = UpdateChecker::new(env!("CARGO_PKG_VERSION"), "xcat", "amemory");
        match checker.check_github().await {
            Ok(Some(info)) => {
                store.set_update_phase(UpdatePhase::Available(info.clone()));
                store.set_update_available(Some(info));
            }
            Ok(None) => {
                store.set_update_phase(UpdatePhase::UpToDate);
            }
            Err(e) => {
                store.set_update_phase(UpdatePhase::Error(e));
            }
        }
    }

    /// Asynchronously download the release asset, track progress, and verify checksum.
    pub async fn execute_download_and_verify(store: AppStore) -> Result<(), String> {
        let info = store
            .update_available
            .get_untracked()
            .ok_or_else(|| "没有可用的更新版本".to_string())?;

        let st = store;
        download_and_prepare_update(&info, move |p| {
            st.set_update_phase(p);
        })
        .await
    }

    /// Prepare installation and restart into the updated binary.
    pub fn apply_and_restart(store: AppStore) -> Result<(), String> {
        let info = store
            .update_available
            .get_untracked()
            .ok_or_else(|| "没有可用的更新版本".to_string())?;
        apply_update_and_restart(&info.asset_name)
    }
}
