//! Update checker resolving releases from GitHub Releases or custom mirror.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Deserialize;

use crate::core::i18n_keys::I18nKey;

const SCHEDULED_UPDATE_CHECK_INTERVAL_HOURS: i64 = 24;

/// Info about the latest available release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// Semver string (e.g. `"0.3.0"`).
    pub latest_version: String,
    /// Markdown release notes.
    pub release_notes: String,
    /// Asset download URL.
    pub download_url: String,
    /// Checksum file or hash string.
    pub checksum_url: String,
    /// Asset filename on disk.
    pub asset_name: String,
    /// Asset size in bytes.
    pub asset_size: u64,
    /// Embedded SHA256 when known.
    pub sha256: Option<String>,
    /// Update channel source.
    pub source: UpdateSource,
}

/// User-selected update channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    /// Auto-detect fastest channel.
    Auto,
    /// Official OSS mirror.
    Oss,
    /// GitHub Releases.
    GitHub,
}

impl UpdateChannel {
    /// Parse from persisted setting string.
    #[must_use]
    pub fn from_setting(value: &str) -> Self {
        match value {
            "oss" => Self::Oss,
            "github" => Self::GitHub,
            _ => Self::Auto,
        }
    }
}

/// Source that provided the update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSource {
    /// OSS mirror.
    Oss,
    /// GitHub.
    GitHub,
}

/// Phase of the update process for UI display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdatePhase {
    /// Idle / no active check.
    Idle,
    /// Checking for new releases.
    Checking,
    /// Already running the latest version.
    UpToDate,
    /// Newer version discovered.
    UpdateAvailable,
    /// Downloading installer binary.
    Downloading {
        /// Download progress 0..=100.
        progress: u8,
    },
    /// Verifying file checksum.
    Verifying,
    /// Preparing platform installer.
    Installing,
    /// Ready to restart into installer.
    ReadyToRestart,
    /// Encountered error during update.
    Error(String),
}

/// Check if periodic update check is due.
#[must_use]
pub fn scheduled_update_check_due(last_check_at: &str, now: DateTime<Utc>) -> bool {
    let trimmed = last_check_at.trim();
    if trimmed.is_empty() {
        return true;
    }
    let Ok(last) = DateTime::parse_from_rfc3339(trimmed) else {
        return true;
    };
    now.signed_duration_since(last.with_timezone(&Utc))
        >= ChronoDuration::hours(SCHEDULED_UPDATE_CHECK_INTERVAL_HOURS)
}

/// Sanitize asset filename.
#[must_use]
pub fn is_safe_asset_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && !name.starts_with('.')
}

/// Open releases page in system web browser.
pub fn open_releases_page(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let wide_url: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            ShellExecuteW(0, op.as_ptr(), wide_url.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
    }
}

/// GitHub release item deserialization.
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// Update checker client.
pub struct UpdateChecker {
    current_version: String,
    repo_owner: String,
    repo_name: String,
}

impl UpdateChecker {
    /// Create new checker for this app.
    #[must_use]
    pub fn new(current_version: &str, repo_owner: &str, repo_name: &str) -> Self {
        Self {
            current_version: current_version.to_string(),
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
        }
    }

    /// Check GitHub releases for updates.
    pub async fn check_github(&self) -> Result<Option<UpdateInfo>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("Amemory/{}", self.current_version))
            .build()
            .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrNetwork.text()))?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.repo_owner, self.repo_name
        );

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrNetwork.text()))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let release: GitHubRelease = resp
            .json()
            .await
            .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrServer.text()))?;

        let latest_ver = release.tag_name.trim_start_matches('v');
        if let (Ok(cur), Ok(latest)) = (
            semver::Version::parse(&self.current_version),
            semver::Version::parse(latest_ver),
        ) {
            if latest <= cur {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }

        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name.ends_with(".exe") || a.name.ends_with(".zip"));

        if let Some(asset) = asset {
            Ok(Some(UpdateInfo {
                latest_version: latest_ver.to_string(),
                release_notes: release.body.unwrap_or_default(),
                download_url: asset.browser_download_url,
                checksum_url: String::new(),
                asset_name: asset.name,
                asset_size: asset.size,
                sha256: None,
                source: UpdateSource::GitHub,
            }))
        } else {
            Ok(None)
        }
    }

    /// Check update according to selected channel.
    pub async fn check_full(&self, _channel: UpdateChannel) -> Result<Option<UpdateInfo>, String> {
        self.check_github().await
    }
}
