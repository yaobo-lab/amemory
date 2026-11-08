//! File downloader with progress reporting and SHA256 checksum verification.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::core::i18n_keys::I18nKey;

/// Download a file from `url` to `dest_path`, reporting 0..=100 progress.
pub async fn download_file(
    url: &str,
    dest_path: &Path,
    expected_size: u64,
    mut on_progress: impl FnMut(u8),
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrDownload.text()))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP error {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(expected_size);
    let mut file = File::create(dest_path)
        .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrInstall.text()))?;

    let mut downloaded: u64 = 0;
    let mut last_pct = 0;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrDownload.text()))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrInstall.text()))?;
        downloaded += chunk.len() as u64;
        if let Some(pct) = (downloaded * 100).checked_div(total_size) {
            let pct = pct.min(100) as u8;
            if pct != last_pct {
                last_pct = pct;
                on_progress(pct);
            }
        }
    }

    file.flush()
        .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrInstall.text()))?;
    on_progress(100);
    Ok(())
}

/// Verify the SHA256 digest of `file_path`.
pub fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<(), String> {
    let mut file =
        File::open(file_path).map_err(|e| format!("{}: {e}", I18nKey::UpdateErrVerify.text()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("{}: {e}", I18nKey::UpdateErrVerify.text()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let actual_hex = format!("{:x}", hasher.finalize());
    if actual_hex.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        Err(format!(
            "{}: expected {}, got {}",
            I18nKey::UpdateErrVerify.text(),
            expected_hex,
            actual_hex
        ))
    }
}
