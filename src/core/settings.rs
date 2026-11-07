//! GUI appearance and interaction settings.

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

use super::paths::project_root;
use super::types::ItemKind;

/// Path where GUI-specific user preferences are stored.
const GUI_SETTINGS_FILE: &str = "etc/gui_settings.json";

/// Window position strategy when summoned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowPositionMode {
    /// Center of the active monitor.
    #[default]
    Center,
    /// Beside or below the current mouse cursor.
    FollowMouse,
    /// Restore last closed position.
    Remember,
}

/// Card height mode for knowledge list items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardHeightMode {
    /// 68px compact cards.
    Compact,
    /// 96px standard cards.
    #[default]
    Standard,
    /// 128px comfortable cards with longer snippets.
    Comfort,
    /// Dynamic height based on content.
    Adaptive,
}

impl CardHeightMode {
    /// Height in logical points for fixed modes.
    #[must_use]
    pub const fn height_px(self) -> f32 {
        match self {
            Self::Compact => 68.0,
            Self::Standard | Self::Adaptive => 96.0,
            Self::Comfort => 128.0,
        }
    }
}

/// Theme appearance preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// Follow operating system appearance.
    #[default]
    System,
    /// Force dark mode.
    Dark,
    /// Force light mode.
    Light,
}

/// User GUI configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct GuiSettings {
    /// Theme appearance.
    pub theme_mode: ThemeMode,
    /// Custom hex accent color (e.g. `"#7ecba3"`).
    pub accent_color: Option<String>,
    /// Custom font family.
    pub font_family: Option<String>,
    /// Font scaling multiplier (0.80 ..= 1.40).
    pub font_scale: f32,
    /// UI language (`"zh"` or `"en"`).
    pub language: String,
    /// Card height preset.
    pub card_height_mode: CardHeightMode,
    /// Window popup positioning mode.
    pub window_position_mode: WindowPositionMode,
    /// Whether to auto-hide the window when it loses focus.
    pub auto_hide_on_blur: bool,
    /// Whether to automatically check for updates periodically.
    pub auto_check_updates: bool,
    /// Preferred update channel (`"auto"`, `"oss"`, `"github"`).
    pub update_channel: String,
    /// Whether operation sound effects are enabled.
    pub sound_effect_enabled: bool,
    /// Sound effect asset name.
    pub sound_effect_name: String,
    /// Sound index (1..=6).
    #[serde(default = "default_sound_idx")]
    pub copy_sound_index: usize,
    /// Whether to play sound on copy.
    #[serde(default = "default_true")]
    pub play_sound_on_copy: bool,
    /// Whether paste on select.
    #[serde(default = "default_true")]
    pub paste_on_select: bool,
    /// Global summon hotkey string.
    #[serde(default = "default_hotkey")]
    pub global_summon_hotkey: String,
    /// Whether the home stats overview banner is visible.
    pub show_stats_overview: bool,
    /// Whether to watch catalog files automatically.
    #[serde(default = "default_true")]
    pub auto_watch_catalog: bool,
    /// Persisted favorite knowledge item IDs.
    #[serde(default)]
    pub favorite_ids: Vec<String>,
    /// Persisted custom tags per item ID.
    #[serde(default)]
    pub custom_tags: BTreeMap<String, Vec<String>>,
    /// User-created filter tags.
    #[serde(default)]
    pub user_tags: Vec<String>,
    /// Visible category filter kinds in the top filter bar.
    #[serde(default = "default_visible_kinds")]
    pub visible_kinds: Vec<ItemKind>,
    /// Window width in points.
    pub window_width: f32,
    /// Window height in points.
    pub window_height: f32,
    /// Last remembered X position.
    pub last_pos_x: Option<f32>,
    /// Last remembered Y position.
    pub last_pos_y: Option<f32>,
}

const fn default_sound_idx() -> usize {
    1
}
const fn default_true() -> bool {
    true
}
fn default_hotkey() -> String {
    "Alt+Space".to_string()
}
fn default_visible_kinds() -> Vec<ItemKind> {
    ItemKind::ALL.to_vec()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            accent_color: None,
            font_family: None,
            font_scale: 1.0,
            language: "zh".to_string(),
            card_height_mode: CardHeightMode::Standard,
            window_position_mode: WindowPositionMode::Center,
            auto_hide_on_blur: false,
            auto_check_updates: true,
            update_channel: "auto".to_string(),
            sound_effect_enabled: true,
            sound_effect_name: "copy_kacha.wav".to_string(),
            copy_sound_index: 1,
            play_sound_on_copy: true,
            paste_on_select: true,
            global_summon_hotkey: "Alt+Space".to_string(),
            show_stats_overview: true,
            auto_watch_catalog: true,
            favorite_ids: Vec::new(),
            custom_tags: BTreeMap::new(),
            user_tags: Vec::new(),
            visible_kinds: default_visible_kinds(),
            window_width: 525.0,
            window_height: 525.0,
            last_pos_x: None,
            last_pos_y: None,
        }
    }
}

impl GuiSettings {
    /// Load GUI settings from disk, or return default.
    #[must_use]
    pub fn load() -> Self {
        let path = project_root().join(GUI_SETTINGS_FILE);
        if path.is_file()
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(settings) = serde_json::from_str::<Self>(&content)
        {
            super::i18n::set_language(&settings.language);
            return settings;
        }
        let def = Self::default();
        super::i18n::set_language(&def.language);
        def
    }

    /// Persist GUI settings to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = project_root().join(GUI_SETTINGS_FILE);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化 GUI 配置失败: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("写入 GUI 配置文件失败: {e}"))
    }

    /// Check whether an item ID is marked as favorite.
    #[must_use]
    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorite_ids.iter().any(|x| x == id)
    }

    /// Toggle favorite status for an item ID and return the new status.
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if let Some(pos) = self.favorite_ids.iter().position(|x| x == id) {
            self.favorite_ids.remove(pos);
            false
        } else {
            self.favorite_ids.push(id.to_string());
            true
        }
    }

    /// Check whether a category filter chip is enabled.
    #[must_use]
    pub fn is_kind_visible(&self, kind: ItemKind) -> bool {
        self.visible_kinds.contains(&kind)
    }

    /// Set whether a category filter chip is enabled.
    pub fn set_kind_visible(&mut self, kind: ItemKind, visible: bool) {
        if visible {
            if !self.visible_kinds.contains(&kind) {
                self.visible_kinds.push(kind);
            }
        } else {
            self.visible_kinds.retain(|&k| k != kind);
        }
    }

    /// Add a user-created tag if not already present.
    pub fn add_user_tag(&mut self, tag: &str) {
        let trimmed = tag.trim();
        if !trimmed.is_empty() && !self.user_tags.iter().any(|t| t == trimmed) {
            self.user_tags.push(trimmed.to_string());
        }
    }

    /// Save custom tags for a specific item ID.
    pub fn set_item_tags(&mut self, id: &str, tags: Vec<String>) {
        for t in &tags {
            self.add_user_tag(t);
        }
        self.custom_tags.insert(id.to_string(), tags);
    }

    /// Merge base item tags with any persisted custom tags for `id`.
    #[must_use]
    pub fn merged_item_tags(&self, id: &str, base_tags: &[String]) -> Vec<String> {
        if let Some(custom) = self.custom_tags.get(id) {
            custom.clone()
        } else {
            base_tags.to_vec()
        }
    }
}
