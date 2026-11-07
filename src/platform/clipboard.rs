//! Clipboard read and write platform integration.

use nsh_core::element::set_clipboard;

/// Standard Windows clipboard format for Unicode text (`CF_UNICODETEXT` = 13).
#[cfg(target_os = "windows")]
const CF_UNICODETEXT: u32 = 13;

/// Set plain text into system clipboard.
pub fn write_text(text: impl Into<String>) {
    set_clipboard(text.into());
}

/// Read current plain text from system clipboard synchronously.
#[must_use]
#[cfg(target_os = "windows")]
pub fn read_text() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    unsafe {
        if OpenClipboard(0) == 0 {
            return None;
        }
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            let _ = CloseClipboard();
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle == 0 {
            let _ = CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle as *mut _) as *const u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let res = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(handle as *mut _);
        let _ = CloseClipboard();
        Some(res)
    }
}

/// Read current plain text from system clipboard synchronously (fallback).
#[must_use]
#[cfg(not(target_os = "windows"))]
pub fn read_text() -> Option<String> {
    None
}
