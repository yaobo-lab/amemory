//! Tab 1: General Settings (Theme, Accent, Font Scale, Language, Window, Auto-hide).

use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::core::settings::{CardHeightMode, ThemeMode, WindowPositionMode};
use crate::platform::window_frame::{DEFAULT_WINDOW_H, DEFAULT_WINDOW_W};
use crate::state::app::AppStore;
use crate::ui::components::slider::stepped_slider;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 1: General appearance and behavior settings.
#[must_use]
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
                        .child(theme_btn(
                            ThemeMode::System,
                            "系统",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(theme_btn(
                            ThemeMode::Dark,
                            "暗色",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(theme_btn(
                            ThemeMode::Light,
                            "亮色",
                            store,
                            accent,
                            divider,
                            text_2,
                        )),
                    theme,
                ),
                // Accent Color
                setting_row(
                    "主题强调色",
                    Some("自定义高亮与选中主色调"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(accent_dot(None, 0x7ecba3, "默认绿", store, divider))
                        .child(accent_dot(
                            Some("#3b82f6"),
                            0x3b82f6,
                            "科技蓝",
                            store,
                            divider,
                        ))
                        .child(accent_dot(
                            Some("#8b5cf6"),
                            0x8b5cf6,
                            "极光紫",
                            store,
                            divider,
                        ))
                        .child(accent_dot(
                            Some("#f59e0b"),
                            0xf59e0b,
                            "琥珀金",
                            store,
                            divider,
                        ))
                        .child(accent_dot(
                            Some("#f43f5e"),
                            0xf43f5e,
                            "珊瑚红",
                            store,
                            divider,
                        )),
                    theme,
                ),
                // Language
                setting_row(
                    "界面语言",
                    Some("切换中文或英文界面显示"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(lang_btn("zh", "简体中文", store, accent, divider, text_2))
                        .child(lang_btn("en", "English", store, accent, divider, text_2)),
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
                            store.update_settings(|s| s.font_scale = val);
                            store.show_toast(format!("已调整界面缩放: {:.0}%", val * 100.0));
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
                        .child(card_height_btn(
                            CardHeightMode::Compact,
                            "紧凑 68px",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(card_height_btn(
                            CardHeightMode::Standard,
                            "标准 96px",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(card_height_btn(
                            CardHeightMode::Comfort,
                            "宽敞 128px",
                            store,
                            accent,
                            divider,
                            text_2,
                        )),
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
                        .child(pos_btn(
                            WindowPositionMode::FollowMouse,
                            "跟随鼠标",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(pos_btn(
                            WindowPositionMode::Center,
                            "屏幕居中",
                            store,
                            accent,
                            divider,
                            text_2,
                        ))
                        .child(pos_btn(
                            WindowPositionMode::Remember,
                            "记住位置",
                            store,
                            accent,
                            divider,
                            text_2,
                        )),
                    theme,
                ),
                // Auto hide on blur
                setting_row(
                    "失去焦点自动隐藏",
                    Some("窗口点击外部时自动最小化到托盘"),
                    render_toggle(settings.get().auto_hide_on_blur, theme, move |next| {
                        store.update_settings(|s| s.auto_hide_on_blur = next);
                        store.show_toast(if next {
                            "已开启失焦自动隐藏"
                        } else {
                            "已关闭失焦自动隐藏"
                        });
                    }),
                    theme,
                ),
                // Auto copy sound
                setting_row(
                    "复制/操作音效",
                    Some("当复制知识卡片时播放提示音"),
                    render_toggle(settings.get().play_sound_on_copy, theme, move |next| {
                        store.update_settings(|s| s.play_sound_on_copy = next);
                        store.show_toast(if next {
                            "已开启操作音效"
                        } else {
                            "已关闭操作音效"
                        });
                    }),
                    theme,
                ),
            ],
        ))
}

fn theme_btn(
    mode: ThemeMode,
    label: &'static str,
    store: AppStore,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().theme_mode == mode {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.theme_mode = mode);
            store.show_toast(format!("已切换主题模式: {label}"));
        })
        .child(
            text(label)
                .text_color(move || {
                    if settings.get().theme_mode == mode {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}

fn accent_dot(
    hex: Option<&'static str>,
    color_u32: u32,
    label: &'static str,
    store: AppStore,
    divider: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .size(px(18.0))
        .rounded(px(9.0))
        .bg(rgb(color_u32))
        .border(px(2.0))
        .border_color(move || {
            let cur = settings.get().accent_color;
            if cur.as_deref() == hex {
                rgb(0xffffff)
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.accent_color = hex.map(str::to_string));
            store.show_toast(format!("已切换强调色: {label}"));
        })
}

fn lang_btn(
    lang_code: &'static str,
    label: &'static str,
    store: AppStore,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().language == lang_code {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.language = lang_code.to_string());
            store.show_toast(format!("已切换语言: {label}"));
        })
        .child(
            text(label)
                .text_color(move || {
                    if settings.get().language == lang_code {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}

fn card_height_btn(
    mode: CardHeightMode,
    label: &'static str,
    store: AppStore,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().card_height_mode == mode {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.card_height_mode = mode);
            store.show_toast(format!("已切换卡片高度: {label}"));
        })
        .child(
            text(label)
                .text_color(move || {
                    if settings.get().card_height_mode == mode {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}

fn pos_btn(
    mode: WindowPositionMode,
    label: &'static str,
    store: AppStore,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().window_position_mode == mode {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.window_position_mode = mode);
            crate::platform::monitor::position_window(mode, DEFAULT_WINDOW_W, DEFAULT_WINDOW_H);
            store.show_toast(format!("已应用窗口位置策略: {label}"));
        })
        .child(
            text(label)
                .text_color(move || {
                    if settings.get().window_position_mode == mode {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}
