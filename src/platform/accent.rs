//! System accent color and theme detection.

/// Get the system accent color hex string, or fallback to default green.
#[must_use]
pub fn system_accent_hex() -> String {
    #[cfg(target_os = "windows")]
    {
        read_windows_accent_color().unwrap_or_else(|| "#7ecba3".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        "#7ecba3".to_string()
    }
}

/// Check if the operating system is currently configured for dark theme.
#[must_use]
pub fn is_system_dark_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        read_windows_is_dark()
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

#[cfg(target_os = "windows")]
fn read_windows_accent_color() -> Option<String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    let subkey = windows_sys::core::w!("Software\\Microsoft\\Windows\\DWM");
    let mut hkey = std::mem::MaybeUninit::uninit();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, hkey.as_mut_ptr()) != 0 {
            return None;
        }
        let hkey = hkey.assume_init();
        let value_name = windows_sys::core::w!("AccentColor");
        let mut data_type = 0u32;
        let mut data = 0u32;
        let mut data_len = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
        let res = RegQueryValueExW(
            hkey,
            value_name,
            null_mut(),
            &raw mut data_type,
            (&raw mut data).cast::<u8>(),
            &raw mut data_len,
        );
        let _ = RegCloseKey(hkey);
        if res == 0 && data_type == REG_DWORD {
            // AccentColor is 0xAABBGGRR
            let r = (data & 0xFF) as u8;
            let g = ((data >> 8) & 0xFF) as u8;
            let b = ((data >> 16) & 0xFF) as u8;
            Some(format!("#{r:02x}{g:02x}{b:02x}"))
        } else {
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn read_windows_is_dark() -> bool {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    let subkey =
        windows_sys::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let mut hkey = std::mem::MaybeUninit::uninit();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, hkey.as_mut_ptr()) != 0 {
            return false;
        }
        let hkey = hkey.assume_init();
        let value_name = windows_sys::core::w!("AppsUseLightTheme");
        let mut data_type = 0u32;
        let mut data = 0u32;
        let mut data_len = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
        let res = RegQueryValueExW(
            hkey,
            value_name,
            null_mut(),
            &raw mut data_type,
            (&raw mut data).cast::<u8>(),
            &raw mut data_len,
        );
        let _ = RegCloseKey(hkey);
        if res == 0 && data_type == REG_DWORD {
            data == 0
        } else {
            false
        }
    }
}
