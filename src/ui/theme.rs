//! Theme tokens and geometry constants replicating Clippi 1:1.

use nsh_core::prelude::*;

use crate::core::settings::ThemeMode;

/// Corner radius for the main window panel.
pub const PANEL_RADIUS: f32 = 12.0;

/// Default transition durations in milliseconds.
pub const VIEW_ANIM_MS: u64 = 180;
/// Duration for toast popup transition in milliseconds.
pub const TOAST_ANIM_MS: u64 = 220;
/// Duration for button expand animation in milliseconds.
pub const BTN_EXPAND_MS: u64 = 240;

/// Complete color palette for dark and light modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippiTheme {
    /// Window and title bar background.
    pub bg: Rgba,
    /// Alias for root panel background.
    pub panel_bg: Rgba,
    /// Card and grouped section surface background.
    pub surface: Rgba,
    /// Alias for card background.
    pub card_bg: Rgba,
    /// Button and card active pressed background.
    pub surface_press: Rgba,
    /// Title bar background.
    pub titlebar_bg: Rgba,
    /// Primary high-contrast text.
    pub text_1: Rgba,
    /// Secondary medium-contrast text and inactive icons.
    pub text_2: Rgba,
    /// Tertiary low-contrast text, placeholders, and shortcuts.
    pub text_3: Rgba,
    /// Primary brand accent color.
    pub accent: Rgba,
    /// Soft translucent accent for selected chips and card backgrounds.
    pub accent_soft: Rgba,
    /// Favorite star and pin indicator color.
    pub fav_color: Rgba,
    /// Destructive action and error indicator color.
    pub danger: Rgba,
    /// 1px divider and border color.
    pub divider: Rgba,
    /// Input border color.
    pub input_border: Rgba,
    /// Hover state background for icon buttons.
    pub btn_hover: Rgba,
    /// Transfer / pin blue indicator color.
    pub transfer_pin_color: Rgba,
    /// Tag chip background.
    pub tag_bg: Rgba,
    /// Tag chip text color.
    pub tag_text: Rgba,
    /// Flyout panel surface background.
    pub panel_surface: Rgba,
    /// Flyout panel input background.
    pub panel_input_bg: Rgba,
    /// Input background.
    pub input_bg: Rgba,
    /// Hairline separator.
    pub panel_sep_line: Rgba,
    /// Floating toast background.
    pub toast_bg: Rgba,
}

impl ClippiTheme {
    /// Default dark theme (1:1 Clippi values).
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            bg: rgb(0x191a1b),
            panel_bg: rgb(0x191a1b),
            surface: rgb(0x232425),
            card_bg: rgb(0x232425),
            surface_press: rgb(0x3a3b3c),
            titlebar_bg: rgb(0x191a1b),
            text_1: rgb(0xeaebec),
            text_2: rgb(0x919496),
            text_3: rgb(0x5f6264),
            accent: rgb(0x7ecba3),
            accent_soft: rgb(0x1a2e24),
            fav_color: rgb(0xd8a155),
            danger: rgb(0xff5f57),
            divider: rgb(0x2b2c2d),
            input_border: rgb(0x2b2c2d),
            btn_hover: rgb(0x2b2c2d),
            transfer_pin_color: rgb(0x3b82f6),
            tag_bg: rgb(0x2c2e2f),
            tag_text: rgb(0xe4e9e6),
            panel_surface: rgb(0x2c2d2e),
            panel_input_bg: rgb(0x1e1f20),
            input_bg: rgb(0x1e1f20),
            panel_sep_line: rgba(0xffffff0d),
            toast_bg: rgba(0x3a3b3de8),
        }
    }

    /// Default light theme (1:1 Clippi values).
    #[must_use]
    pub const fn light() -> Self {
        Self {
            bg: rgb(0xf2f3f8),
            panel_bg: rgb(0xf2f3f8),
            surface: rgb(0xffffff),
            card_bg: rgb(0xffffff),
            surface_press: rgb(0xebedf5),
            titlebar_bg: rgb(0xf2f3f8),
            text_1: rgb(0x1a1c2e),
            text_2: rgb(0x7c809a),
            text_3: rgb(0xb4b9ca),
            accent: rgb(0x6ab890),
            accent_soft: rgb(0xe8f5ee),
            fav_color: rgb(0xd8a155),
            danger: rgb(0xff5f57),
            divider: rgb(0xe6e8f0),
            input_border: rgb(0xe6e8f0),
            btn_hover: rgb(0xf0f1f8),
            transfer_pin_color: rgb(0x3b82f6),
            tag_bg: rgb(0xe6e8f0),
            tag_text: rgb(0x767e7a),
            panel_surface: rgb(0xffffff),
            panel_input_bg: rgb(0xf5f6fc),
            input_bg: rgb(0xf5f6fc),
            panel_sep_line: rgba(0x00000010),
            toast_bg: rgba(0xfffffff0),
        }
    }

    /// Resolve theme from user setting mode.
    #[must_use]
    pub fn resolve(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
            ThemeMode::System => {
                if crate::platform::is_system_dark_mode() {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }
}
