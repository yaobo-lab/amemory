//! Theme tokens and geometry constants replicating Clippi 1:1.

use nsh_core::prelude::*;

use crate::core::settings::ThemeMode;

/// Corner radius for the main window panel.
pub const PANEL_RADIUS: f32 = 12.0;

/// Default transition durations in milliseconds.
pub const VIEW_ANIM_MS: u64 = 180;
pub const TOAST_ANIM_MS: u64 = 220;
pub const BTN_EXPAND_MS: u64 = 240;

/// Complete color palette for dark and light modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippiTheme {
    /// Window and title bar background.
    pub bg: Rgba,
    /// Card and grouped section surface background.
    pub surface: Rgba,
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
    /// Hairline separator.
    pub panel_sep_line: Rgba,
    /// Floating toast background.
    pub toast_bg: Rgba,
}

impl ClippiTheme {
    /// Default dark theme (1:1 Clippi values).
    #[must_use]
    pub fn dark() -> Self {
        Self {
            bg: rgb(0x19, 0x1a, 0x1b),
            surface: rgb(0x23, 0x24, 0x25),
            surface_press: rgb(0x3a, 0x3b, 0x3c),
            titlebar_bg: rgb(0x19, 0x1a, 0x1b),
            text_1: rgb(0xea, 0xeb, 0xec),
            text_2: rgb(0x91, 0x94, 0x96),
            text_3: rgb(0x5f, 0x62, 0x64),
            accent: rgb(0x7e, 0xcb, 0xa3),
            accent_soft: rgb(0x1a, 0x2e, 0x24),
            fav_color: rgb(0xd8, 0xa1, 0x55),
            danger: rgb(0xff, 0x5f, 0x57),
            divider: rgb(0x2b, 0x2c, 0x2d),
            btn_hover: rgb(0x2b, 0x2c, 0x2d),
            transfer_pin_color: rgb(0x3b, 0x82, 0xf6),
            tag_bg: rgb(0x2c, 0x2e, 0x2f),
            tag_text: rgb(0xe4, 0xe9, 0xe6),
            panel_surface: rgb(0x2c, 0x2d, 0x2e),
            panel_input_bg: rgb(0x1e, 0x1f, 0x20),
            panel_sep_line: rgba(0xff, 0xff, 0xff, 0x0d),
            toast_bg: rgba(0x3a, 0x3b, 0x3d, 0xe8),
        }
    }

    /// Default light theme (1:1 Clippi values).
    #[must_use]
    pub fn light() -> Self {
        Self {
            bg: rgb(0xf2, 0xf3, 0xf8),
            surface: rgb(0xff, 0xff, 0xff),
            surface_press: rgb(0xeb, 0xed, 0xf5),
            titlebar_bg: rgb(0xf2, 0xf3, 0xf8),
            text_1: rgb(0x1a, 0x1c, 0x2e),
            text_2: rgb(0x7c, 0x80, 0x9a),
            text_3: rgb(0xb4, 0xb9, 0xca),
            accent: rgb(0x6a, 0xb8, 0x90),
            accent_soft: rgb(0xe8, 0xf5, 0xee),
            fav_color: rgb(0xd8, 0xa1, 0x55),
            danger: rgb(0xff, 0x5f, 0x57),
            divider: rgb(0xe6, 0xe8, 0xf0),
            btn_hover: rgb(0xf0, 0xf1, 0xf8),
            transfer_pin_color: rgb(0x3b, 0x82, 0xf6),
            tag_bg: rgb(0xe6, 0xe8, 0xf0),
            tag_text: rgb(0x76, 0x7e, 0x7a),
            panel_surface: rgb(0xff, 0xff, 0xff),
            panel_input_bg: rgb(0xf5, 0xf6, 0xfc),
            panel_sep_line: rgba(0x00, 0x00, 0x00, 0x10),
            toast_bg: rgba(0xff, 0xff, 0xff, 0xf0),
        }
    }

    /// Resolve theme from user setting mode.
    #[must_use]
    pub fn resolve(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::System => Self::dark(), // default system to dark
        }
    }
}
