//! Platform installer preparation and invocation.

use std::path::{Path, PathBuf};

use crate::core::i18n_keys::I18nKey;

/// Temp directory where updates are downloaded.
#[must_use]
pub fn update_temp_dir() -> PathBuf {
    std::env::temp_dir().join("amemory-update")
}

/// Cleanup leftover files from previous update attempts.
pub fn cleanup_temp() {
    let dir = update_temp_dir();
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Launch prepared platform installer.
pub fn launch_prepared_update(installer_path: &Path) -> Result<(), String> {
    if !installer_path.is_file() {
        return Err(format!(
            "{}: {}",
            I18nKey::UpdateErrInstall.text(),
            installer_path.display()
        ));
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let exe: Vec<u16> = installer_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let dir: Option<Vec<u16>> = installer_path.parent().map(|d| {
            d.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
        });
        let dir_ptr = dir.as_ref().map_or(std::ptr::null(), std::vec::Vec::as_ptr);

        let ret = unsafe {
            ShellExecuteW(0, op.as_ptr(), exe.as_ptr(), std::ptr::null(), dir_ptr, SW_SHOWNORMAL)
        } as isize;

        if ret > 32 {
            std::process::exit(0);
        } else {
            Err(format!("{}: code {ret}", I18nKey::UpdateErrLaunch.text()))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = installer_path;
        Err("Unsupported platform for self-update".into())
    }
}
