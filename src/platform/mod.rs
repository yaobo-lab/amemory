//! Platform abstraction layer for window, monitor, tray, hotkey, and clipboard.

pub mod accent;
pub mod clipboard;
pub mod hotkey;
pub mod monitor;
pub mod system_font;
pub mod tag_window;
pub mod tray;
pub mod window_frame;

pub use accent::{is_system_dark_mode, system_accent_hex};
