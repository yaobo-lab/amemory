//! Static assets: font glyphs, icons, and image definitions.

use std::sync::OnceLock;
use base64::prelude::*;

/// Embedded PNG bytes of the application logo without text.
pub const LOGO_NOTEXT_PNG_BYTES: &[u8] = include_bytes!("../../assets/LOGO_notext.png");

/// Embedded ICO bytes of the application icon.
pub const LOGO_NOTEXT_ICO_BYTES: &[u8] = include_bytes!("../../assets/LOGO_notext.ico");

/// Base64 data URI of the logo for NSH image element rendering.
#[must_use]
pub fn logo_notext_data_uri() -> &'static str {
    static URI: OnceLock<String> = OnceLock::new();
    URI.get_or_init(|| {
        format!("data:image/png;base64,{}", BASE64_STANDARD.encode(LOGO_NOTEXT_PNG_BYTES))
    })
}

/// Common icon glyph unicode characters defined in iconfont.ttf.
pub mod glyphs {
    /// Search magnifying glass
    pub const SEARCH: &str = "\u{e688}";
    /// Plus symbol for new item
    pub const PLUS: &str = "\u{e61b}";
    /// Tag filter symbol
    pub const TAG: &str = "\u{e886}";
    /// Favorite star
    pub const STAR: &str = "\u{e630}";
    /// Pin window always on top
    pub const PIN: &str = "\u{e633}";
    /// Settings gear
    pub const SETTINGS: &str = "\u{e6b6}";
    /// Sync / refresh arrows
    pub const SYNC: &str = "\u{e782}";
    /// Back arrow
    pub const BACK: &str = "\u{e62b}";
    /// Checkmark
    pub const CHECK: &str = "\u{e61f}";
    /// Unchecked box
    pub const UNCHECK: &str = "\u{e831}";
    /// Copy document
    pub const COPY: &str = "\u{e65c}";
    /// Edit pencil
    pub const EDIT: &str = "\u{e603}";
    /// Trash delete
    pub const DELETE: &str = "\u{e60e}";
    /// Folder open
    pub const FOLDER: &str = "\u{e664}";
    /// Import document
    pub const IMPORT: &str = "\u{e649}";
    /// Find and replace
    pub const FIND_REPLACE: &str = "\u{e64c}";
    /// GitHub mark
    pub const GITHUB: &str = "\u{ea0a}";
    /// Skill icon (bulb / brain)
    pub const SKILL: &str = "\u{e7b0}";
    /// Book icon
    pub const BOOK: &str = "\u{e658}";
    /// Memory icon (chip / storage)
    pub const MEMORY: &str = "\u{e665}";
    /// Document file icon
    pub const DOCUMENT: &str = "\u{e65a}";
    /// Script code icon
    pub const SCRIPT: &str = "\u{e63b}";
    /// Note reference icon
    pub const NOTE: &str = "\u{e632}";
    /// All categories icon
    pub const ALL: &str = "\u{e68c}";
    /// Stats chart icon
    pub const STATS: &str = "\u{e64a}";
}
