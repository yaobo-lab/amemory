//! Tab 1: General Settings (Theme, Accent, Font Scale, Language, Window, Auto-hide).

use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::core::settings::{CardHeightMode, ThemeMode, WindowPositionMode};
use crate::state::app::AppStore;
use crate::ui::components::slider::stepped_slider;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 1: General appearance and behavior settings.
pub fn render_general_tab(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let settings = store.settings;
    let accent = theme.accent;
    let divider = theme.divider;
    let text_2 = theme.text_2;

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        // ── Group 1: Appearance ──
        .child(settings_group(
            "外观设置 (Appearance)",
            theme,
            vec![
                // Theme Mode
                setting_row(
                    "主题模式",
                    Some("跟随系统、明亮或暗黑外观"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(theme_btn(ThemeMode::System, "系统", settings, accent, divider, text_2))
                        .child(theme_btn(ThemeMode::Dark, "暗色", settings, accent, divider, text_2))
                        .child(theme_btn(ThemeMode::Light, "亮色", settings, accent, divider, text_2)),
                    theme,
                ),
                // Font Scale Slider
                setting_row(
                    "界面缩放",
                    Some("调整文字与卡片排版缩放 (80% - 140%)"),
                    stepped_slider(
                        0.8,
                        1.4,
                        0.1,
                        settings.get().font_scale,
                        theme,
                        move |val| {
                            settings.update(|s| s.font_scale = val);
                        },
                    ),
                    theme,
                ),
                // Card Height Mode
                setting_row(
                    "卡片高度",
                    Some("知识项列表卡片默认高度"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(card_height_btn(CardHeightMode::Compact, "紧凑 68px", settings, accent, divider, text_2))
                        .child(card_height_btn(CardHeightMode::Standard, "标准 96px", settings, accent, divider, text_2))
                        .child(card_height_btn(CardHeightMode::Comfort, "宽敞 128px", settings, accent, divider, text_2)),
                    theme,
                ),
            ],
        ))
        // ── Group 2: Window & Behavior ──
        .child(settings_group(
            "窗口与行为 (Window & Behavior)",
            theme,
            vec![
                // Window Position Mode
                setting_row(
                    "呼出位置",
                    Some("快捷键唤起主窗口时的对齐基准"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(pos_btn(WindowPositionMode::FollowMouse, "跟随鼠标", settings, accent, divider, text_2))
                        .child(pos_btn(WindowPositionMode::Center, "屏幕居中", settings, accent, divider, text_2))
                        .child(pos_btn(WindowPositionMode::Remember, "记住位置", settings, accent, divider, text_2)),
                    theme,
                ),
                // Auto hide on blur
                setting_row(
                    "失去焦点自动隐藏",
                    Some("窗口点击外部时自动最小化到托盘"),
                    render_toggle(
                        settings.get().auto_hide_on_blur,
                        theme,
                        move |next| {
                            settings.update(|s| s.auto_hide_on_blur = next);
                        },
                    ),
                    theme,
                ),
                // Auto copy sound
                setting_row(
                    "复制/操作音效",
                    Some("当复制知识卡片时播放提示音"),
                    render_toggle(
                        settings.get().play_sound_on_copy,
                        theme,
                        move |next| {
                            settings.update(|s| s.play_sound_on_copy = next);
                        },
                    ),
                    theme,
                ),
            ],
        ))
}

fn theme_btn(
    mode: ThemeMode,
    label: &'static str,
    settings: Signal<crate::core::settings::GuiSettings>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if settings.get().theme_mode == mode { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            settings.update(|s| s.theme_mode = mode);
        })
        .child(
            text(label)
                .text_color(move || if settings.get().theme_mode == mode { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}

fn card_height_btn(
    mode: CardHeightMode,
    label: &'static str,
    settings: Signal<crate::core::settings::GuiSettings>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if settings.get().card_height_mode == mode { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            settings.update(|s| s.card_height_mode = mode);
        })
        .child(
            text(label)
                .text_color(move || if settings.get().card_height_mode == mode { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}

fn pos_btn(
    mode: WindowPositionMode,
    label: &'static str,
    settings: Signal<crate::core::settings::GuiSettings>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if settings.get().window_position_mode == mode { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            settings.update(|s| s.window_position_mode = mode);
        })
        .child(
            text(label)
                .text_color(move || if settings.get().window_position_mode == mode { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}
