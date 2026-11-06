//! Clipboard read and write platform integration.

use nsh_core::element::{clipboard_string, set_clipboard};

/// Set plain text into system clipboard.
pub fn write_text(text: impl Into<String>) {
    set_clipboard(text.into());
}

/// Read current plain text from system clipboard.
#[must_use]
pub fn read_text() -> Option<String> {
    clipboard_string()
}
