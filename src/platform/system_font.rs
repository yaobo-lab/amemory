//! System font detection and fallback lists.

/// Standard default UI font family cascade.
#[must_use]
pub fn default_font_family() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Microsoft YaHei UI, Segoe UI, sans-serif";
    #[cfg(target_os = "macos")]
    return "PingFang SC, -apple-system, sans-serif";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return "Noto Sans CJK SC, sans-serif";
}

/// Standard monospaced font family cascade.
#[must_use]
pub fn default_mono_font_family() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Cascadia Code, Consolas, Courier New, monospace";
    #[cfg(target_os = "macos")]
    return "Menlo, Monaco, monospace";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return "monospace";
}
