//! Static assets: font glyphs, icons, and image definitions.

use base64::prelude::*;
use std::sync::OnceLock;

/// Embedded PNG bytes of the application logo without text.
pub const LOGO_NOTEXT_PNG_BYTES: &[u8] = include_bytes!("../../assets/LOGO_notext.png");

/// Embedded ICO bytes of the application icon.
pub const LOGO_NOTEXT_ICO_BYTES: &[u8] = include_bytes!("../../assets/LOGO_notext.ico");

/// Embedded TTF bytes of `iconfont.ttf` for custom icon glyph rendering.
pub const ICONFONT_TTF_BYTES: &[u8] = include_bytes!("../../assets/fonts/iconfont.ttf");

/// Base64 data URI of the logo for NSH image element rendering.
#[must_use]
pub fn logo_notext_data_uri() -> &'static str {
    static URI: OnceLock<String> = OnceLock::new();
    URI.get_or_init(|| {
        format!(
            "data:image/png;base64,{}",
            BASE64_STANDARD.encode(LOGO_NOTEXT_PNG_BYTES)
        )
    })
}

/// Common icon glyph unicode characters defined in `assets/fonts/iconfont.ttf`.
pub mod glyphs {
    /// Search magnifying glass (`.icon-icon-sousuo`)
    pub const SEARCH: &str = "\u{e688}";
    /// Plus symbol for new item
    pub const PLUS: &str = "＋";
    /// Tag filter symbol (`.icon-24gf-tags3`)
    pub const TAG: &str = "\u{e886}";
    /// Favorite star (`.icon-buoumaotubiao44`)
    pub const STAR: &str = "\u{e630}";
    /// Pin window always on top (`.icon-guding1`)
    pub const PIN: &str = "\u{e633}";
    /// Settings gear (`.icon-shezhi`)
    pub const SETTINGS: &str = "\u{e6b6}";
    /// Sync / refresh arrows (`.icon-tihuan`)
    pub const SYNC: &str = "\u{e6ed}";
    /// Back arrow (`.icon-fanhui`)
    pub const BACK: &str = "\u{e62b}";
    /// Checkmark (`.icon-xuanzhongyuandian`)
    pub const CHECK: &str = "\u{e61f}";
    /// Unchecked box (`.icon-weixuanzhongyuandian`)
    pub const UNCHECK: &str = "\u{e831}";
    /// Copy document (`.icon-fuzhi`)
    pub const COPY: &str = "\u{e7e1}";
    /// Edit pencil (`.icon-bianji`)
    pub const EDIT: &str = "\u{e679}";
    /// Trash delete (`.icon-shanchu2`)
    pub const DELETE: &str = "\u{e696}";
    /// Folder open (`.icon-wenjianjiazhankai`)
    pub const FOLDER: &str = "\u{e60f}";
    /// Import / download document (`.icon-xiazai`)
    pub const IMPORT: &str = "\u{e7c8}";
    /// Find and replace (`.icon-chazhaotihuan`)
    pub const FIND_REPLACE: &str = "\u{e64c}";
    /// GitHub mark (`.icon-GitHub`)
    pub const GITHUB: &str = "\u{ea0a}";
    /// Skill icon (`.icon-kuaijiejian`)
    pub const SKILL: &str = "\u{e66b}";
    /// Book icon (`.icon-24gf-phoneBook`)
    pub const BOOK: &str = "\u{e966}";
    /// Memory icon (`.icon-folder-cloud-fill`)
    pub const MEMORY: &str = "\u{e794}";
    /// Document file icon (`.icon-quanwenpaiban`)
    pub const DOCUMENT: &str = "\u{e6db}";
    /// Script code icon (`.icon-filetobase64`)
    pub const SCRIPT: &str = "\u{e66e}";
    /// Note reference icon (`.icon-beizhu2`)
    pub const NOTE: &str = "\u{e6fa}";
    /// All categories icon (`.icon-zuhe`)
    pub const ALL: &str = "\u{e61e}";
    /// Stats chart icon (`.icon-ee-pushline-report-level`)
    pub const STATS: &str = "\u{e638}";
}
