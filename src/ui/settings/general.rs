//! Tab 1: General Settings (Theme, Accent, Font Scale, Language, Window, Auto-hide).

use nsh_core::prelude::*;

use super::{setting_row_tr, settings_group_reactive};
use crate::core::i18n_keys::I18nKey;
use crate::core::settings::{CardHeightMode, ThemeMode, WindowPositionMode};
use crate::platform::window_frame::{DEFAULT_WINDOW_H, DEFAULT_WINDOW_W};
use crate::state::app::AppStore;
use crate::ui::components::slider::stepped_slider;
use crate::ui::components::toggle::render_store_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 1: General appearance and behavior settings.
#[must_use]
pub fn render_general_tab(store: AppStore, _theme: ClippiTheme) -> impl IntoElement {
    let settings = store.settings;

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        // ── Group 1: Appearance ──
        .child(settings_group_reactive(
            move || store.tr(I18nKey::AppearanceGroup).to_string(),
            store,
            vec![
                // 1. Theme Mode
                setting_row_tr(
                    move || store.tr(I18nKey::ThemeMode).to_string(),
                    Some(move || store.tr(I18nKey::ThemeDesc).to_string()),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(theme_btn(ThemeMode::System, I18nKey::ThemeSystem, store))
                        .child(theme_btn(ThemeMode::Dark, I18nKey::ThemeDark, store))
                        .child(theme_btn(ThemeMode::Light, I18nKey::ThemeLight, store)),
                    store,
                ),
                // 2. Accent Color
                setting_row_tr(
                    move || store.tr(I18nKey::AccentColor).to_string(),
                    Some(move || store.tr(I18nKey::AccentDesc).to_string()),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(accent_dot(None, 0x7ecba3, I18nKey::AccentGreen, store))
                        .child(accent_dot(
                            Some("#3b82f6"),
                            0x3b82f6,
                            I18nKey::AccentBlue,
                            store,
                        ))
                        .child(accent_dot(
                            Some("#8b5cf6"),
                            0x8b5cf6,
                            I18nKey::AccentPurple,
                            store,
                        ))
                        .child(accent_dot(
                            Some("#f59e0b"),
                            0xf59e0b,
                            I18nKey::AccentAmber,
                            store,
                        ))
                        .child(accent_dot(
                            Some("#f43f5e"),
                            0xf43f5e,
                            I18nKey::AccentCoral,
                            store,
                        )),
                    store,
                ),
                // 3. Language
                setting_row_tr(
                    move || store.tr(I18nKey::Language).to_string(),
                    Some(move || store.tr(I18nKey::LangDesc).to_string()),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(lang_btn("zh", I18nKey::LangZh, store))
                        .child(lang_btn("en", I18nKey::LangEn, store)),
                    store,
                ),
                // 4. Font / UI Scale Slider
                setting_row_tr(
                    move || store.tr(I18nKey::FontScale).to_string(),
                    Some(move || store.tr(I18nKey::FontScaleDesc).to_string()),
                    stepped_slider(
                        0.8,
                        1.4,
                        0.1,
                        settings.get_untracked().font_scale,
                        store,
                        move |val| {
                            store.update_settings(|s| s.font_scale = val);
                            store.show_toast(format!("已调整界面缩放: {:.0}%", val * 100.0));
                        },
                    ),
                    store,
                ),
                // 5. Card Height Mode
                setting_row_tr(
                    move || store.tr(I18nKey::CardHeightMode).to_string(),
                    Some(move || store.tr(I18nKey::CardHeightDesc).to_string()),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(card_height_btn(
                            CardHeightMode::Compact,
                            I18nKey::CardHeightCompact,
                            store,
                        ))
                        .child(card_height_btn(
                            CardHeightMode::Standard,
                            I18nKey::CardHeightStandard,
                            store,
                        ))
                        .child(card_height_btn(
                            CardHeightMode::Comfort,
                            I18nKey::CardHeightComfort,
                            store,
                        )),
                    store,
                ),
            ],
        ))
        // ── Group 2: Window & Behavior ──
        .child(settings_group_reactive(
            move || store.tr(I18nKey::WindowBehaviorGroup).to_string(),
            store,
            vec![
                // 6. Window Position Mode
                setting_row_tr(
                    move || store.tr(I18nKey::WindowPositionMode).to_string(),
                    Some(move || store.tr(I18nKey::WindowPosDesc).to_string()),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(pos_btn(
                            WindowPositionMode::FollowMouse,
                            I18nKey::PosFollowMouse,
                            store,
                        ))
                        .child(pos_btn(
                            WindowPositionMode::Center,
                            I18nKey::PosCenter,
                            store,
                        ))
                        .child(pos_btn(
                            WindowPositionMode::Remember,
                            I18nKey::PosRemember,
                            store,
                        )),
                    store,
                ),
                // 7. Auto hide on blur
                setting_row_tr(
                    move || store.tr(I18nKey::AutoHideOnBlur).to_string(),
                    Some(move || store.tr(I18nKey::AutoHideDesc).to_string()),
                    render_store_toggle(
                        store,
                        settings.get_untracked().auto_hide_on_blur,
                        move |next| {
                            store.update_settings(|s| s.auto_hide_on_blur = next);
                            store.show_toast(if next {
                                "已开启失焦自动隐藏"
                            } else {
                                "已关闭失焦自动隐藏"
                            });
                        },
                    ),
                    store,
                ),
                // 8. Auto copy sound
                setting_row_tr(
                    move || store.tr(I18nKey::CopySoundTitle).to_string(),
                    Some(move || store.tr(I18nKey::CopySoundDesc).to_string()),
                    render_store_toggle(
                        store,
                        settings.get_untracked().play_sound_on_copy,
                        move |next| {
                            store.update_settings(|s| s.play_sound_on_copy = next);
                            store.show_toast(if next {
                                "已开启操作音效"
                            } else {
                                "已关闭操作音效"
                            });
                        },
                    ),
                    store,
                ),
            ],
        ))
}

fn theme_btn(mode: ThemeMode, key: I18nKey, store: AppStore) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().theme_mode == mode {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.theme_mode = mode);
            store.show_toast(format!("已切换主题模式: {}", key.text()));
        })
        .child(
            text(move || store.tr(key))
                .text_color(move || {
                    if settings.get().theme_mode == mode {
                        rgb(0xffffff)
                    } else {
                        store.theme().text_2
                    }
                })
                .text_xs(),
        )
}

fn accent_dot(
    hex: Option<&'static str>,
    color_u32: u32,
    key: I18nKey,
    store: AppStore,
) -> impl IntoElement {
    let settings = store.settings;
    view()
        .size(px(20.0))
        .rounded(px(10.0))
        .bg(rgb(color_u32))
        .border(move || {
            if settings.get().accent_color.as_deref() == hex {
                px(2.5)
            } else {
                px(1.0)
            }
        })
        .border_color(move || {
            if settings.get().accent_color.as_deref() == hex {
                store.theme().text_1
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .items_center()
        .justify_center()
        .on_press(move || {
            store.update_settings(|s| s.accent_color = hex.map(str::to_string));
            store.show_toast(format!("已切换强调色: {}", key.text()));
        })
        .child(when(
            move || settings.get().accent_color.as_deref() == hex,
            move || view().size(px(6.0)).rounded(px(3.0)).bg(rgb(0xffffff)),
        ))
}

fn lang_btn(lang_code: &'static str, key: I18nKey, store: AppStore) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().language == lang_code {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.set_language(lang_code);
            let msg = if lang_code.starts_with("en") {
                format!("Language switched to: {}", key.text())
            } else {
                format!("已切换语言: {}", key.text())
            };
            store.show_toast(msg);
        })
        .child(
            text(move || store.tr(key))
                .text_color(move || {
                    if settings.get().language == lang_code {
                        rgb(0xffffff)
                    } else {
                        store.theme().text_2
                    }
                })
                .text_xs(),
        )
}

fn card_height_btn(mode: CardHeightMode, key: I18nKey, store: AppStore) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().card_height_mode == mode {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.card_height_mode = mode);
            store.show_toast(format!("已切换卡片高度: {}", key.text()));
        })
        .child(
            text(move || store.tr(key))
                .text_color(move || {
                    if settings.get().card_height_mode == mode {
                        rgb(0xffffff)
                    } else {
                        store.theme().text_2
                    }
                })
                .text_xs(),
        )
}

fn pos_btn(mode: WindowPositionMode, key: I18nKey, store: AppStore) -> impl IntoElement {
    let settings = store.settings;
    view()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(move || {
            if settings.get().window_position_mode == mode {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.window_position_mode = mode);
            crate::platform::monitor::position_window(mode, DEFAULT_WINDOW_W, DEFAULT_WINDOW_H);
            store.show_toast(format!("已应用窗口位置策略: {}", key.text()));
        })
        .child(
            text(move || store.tr(key))
                .text_color(move || {
                    if settings.get().window_position_mode == mode {
                        rgb(0xffffff)
                    } else {
                        store.theme().text_2
                    }
                })
                .text_xs(),
        )
}
