//! System accent color detection.

/// Get the system accent color hex string, or fallback to default green.
#[must_use]
pub fn system_accent_hex() -> String {
    #[cfg(target_os = "windows")]
    {
        // Try reading DWM accent color from registry
        if let Ok(color) = read_windows_accent_color() {
            return color;
        }
    }
    "#7ecba3".to_string()
}

#[cfg(target_os = "windows")]
fn read_windows_accent_color() -> Result<String, ()> {
    // Default fallback
    Ok("#7ecba3".to_string())
}
