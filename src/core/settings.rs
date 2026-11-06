//! GUI appearance and interaction settings.

use std::fs;

use serde::{Deserialize, Serialize};

use super::paths::project_root;

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
            Self::Standard => 96.0,
            Self::Comfort => 128.0,
            Self::Adaptive => 96.0,
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
pub struct GuiSettings {
    /// Theme appearance.
    pub theme_mode: ThemeMode,
    /// Custom hex accent color (e.g. `"#7ecba3"`).
    pub accent_color: Option<String>,
    /// Custom font family.
    pub font_family: Option<String>,
    /// Font scaling multiplier (0.85 ..= 1.30).
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
    /// Sound index (1..=5).
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
    /// Window width in points.
    pub window_width: f32,
    /// Window height in points.
    pub window_height: f32,
    /// Last remembered X position.
    pub last_pos_x: Option<f32>,
    /// Last remembered Y position.
    pub last_pos_y: Option<f32>,
}

fn default_sound_idx() -> usize { 1 }
fn default_true() -> bool { true }
fn default_hotkey() -> String { "Alt+Space".to_string() }

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
            window_width: 324.0,
            window_height: 480.0,
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
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<Self>(&content) {
                    return settings;
                }
            }
        }
        Self::default()
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
}
