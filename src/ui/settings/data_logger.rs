//! Tab 5: Logging, Diagnostics and Sound Settings.

use std::path::Path;

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::assets::glyphs;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 5: Logging, diagnostics and sound settings.
#[must_use]
pub fn render_data_logger_tab(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let settings = store.settings;
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let accent = theme.accent;
    let divider = theme.divider;

    let cfg = store.app_config.get();
    let log_level = signal(cfg.logger.level.to_ascii_uppercase());
    let log_dir = signal(cfg.logger.dir);
    let console_output = signal(cfg.logger.console);

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(settings_group(
            "日志与诊断输出 (Diagnostics & Logging)",
            theme,
            vec![
                setting_row(
                    "运行日志级别",
                    Some("控制文件输出与后台调试详细度"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(level_btn("WARN", log_level, store, accent, divider, text_1))
                        .child(level_btn("INFO", log_level, store, accent, divider, text_1))
                        .child(level_btn(
                            "DEBUG", log_level, store, accent, divider, text_1,
                        ))
                        .child(level_btn(
                            "TRACE", log_level, store, accent, divider, text_1,
                        )),
                    theme,
                ),
                setting_row(
                    "日志文件路径",
                    Some("日志归档与滚动存储目录"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            input()
                                .value(move || log_dir.get())
                                .on_change(move |v: &str| {
                                    let val = v.to_string();
                                    log_dir.set(val.clone());
                                    if !val.trim().is_empty() {
                                        store.update_app_config(move |c| {
                                            c.logger.dir = val;
                                        });
                                    }
                                })
                                .w(px(136.0))
                                .h(px(24.0))
                                .rounded(px(4.0))
                                .border(px(1.0))
                                .border_color(input_border)
                                .bg(card_bg)
                                .text_xs()
                                .text_color(text_1),
                        )
                        .child(
                            view()
                                .size(px(24.0))
                                .rounded(px(4.0))
                                .bg(divider)
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .on_press(move || {
                                    let dir = log_dir.get();
                                    crate::services::amemory_runtime::open_path_in_explorer(
                                        Path::new(&dir),
                                    );
                                })
                                .child(
                                    text(glyphs::FOLDER)
                                        .font_family("iconfont")
                                        .text_xs()
                                        .text_color(text_1),
                                ),
                        ),
                    theme,
                ),
                setting_row(
                    "输出至标准控制台",
                    Some("终端启动模式下打印实时日志流"),
                    render_toggle(console_output.get(), theme, move |next| {
                        console_output.set(next);
                        store.update_app_config(move |c| {
                            c.logger.console = next;
                        });
                    }),
                    theme,
                ),
            ],
        ))
        .child(settings_group(
            "提示音效配置 (Audio Feedback)",
            theme,
            vec![
                setting_row(
                    "音效方案",
                    Some("选择复制知识卡片时播放的音效"),
                    view()
                        .flex_row()
                        .gap(px(4.0))
                        .child(sound_btn("经典滴答", 1, store, accent, divider, text_1))
                        .child(sound_btn("清脆按键", 2, store, accent, divider, text_1))
                        .child(sound_btn("柔和水滴", 3, store, accent, divider, text_1)),
                    theme,
                ),
                setting_row(
                    "试听音效",
                    Some("立即测试当前选择的提示音"),
                    view()
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(4.0))
                        .bg(accent)
                        .cursor_pointer()
                        .on_press(move || {
                            crate::services::copy_sound::play_copy_sound(
                                settings.get().copy_sound_index,
                            );
                        })
                        .child(
                            text("▶ 试听")
                                .text_color(rgb(0xffffff))
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        ),
                    theme,
                ),
            ],
        ))
}

fn level_btn(
    lvl: &'static str,
    current: Signal<String>,
    store: AppStore,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if current.get() == lvl {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            current.set(lvl.to_string());
            store.update_app_config(|c| {
                c.logger.level = lvl.to_ascii_lowercase();
            });
            store.show_toast(format!("日志级别已设置为 {lvl}"));
        })
        .child(
            text(lvl)
                .text_color(move || {
                    if current.get() == lvl {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}

fn sound_btn(
    name: &'static str,
    idx: usize,
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
            if settings.get().copy_sound_index == idx {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.copy_sound_index = idx);
            crate::services::copy_sound::play_copy_sound(idx);
        })
        .child(
            text(name)
                .text_color(move || {
                    if settings.get().copy_sound_index == idx {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}
