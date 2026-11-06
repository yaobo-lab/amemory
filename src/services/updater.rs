//! Update orchestration coordinating download, verification, and installation.

use super::downloader::{download_file, verify_sha256};
use super::install::{cleanup_temp, launch_prepared_update, update_temp_dir};
use super::update::{UpdateChecker, UpdateInfo, UpdatePhase, is_safe_asset_name};

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
    phase_callback(UpdatePhase::Downloading { progress: 0 });
    download_file(
        &info.download_url,
        &dest_path,
        info.asset_size,
        |progress| {
            phase_callback(UpdatePhase::Downloading { progress });
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
