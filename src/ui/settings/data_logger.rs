//! Tab 5: Logging, Diagnostics and Sound Settings.

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 5: Logging, diagnostics and sound settings.
pub fn render_data_logger_tab(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let settings = store.settings;
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let accent = theme.accent;
    let divider = theme.divider;

    let log_level = signal("INFO".to_string());
    let log_dir = signal("logs/amemory.log".to_string());
    let console_output = signal(true);

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
                        .child(level_btn("INFO", log_level, accent, divider, text_1))
                        .child(level_btn("DEBUG", log_level, accent, divider, text_1))
                        .child(level_btn("TRACE", log_level, accent, divider, text_1)),
                    theme,
                ),
                setting_row(
                    "日志文件路径",
                    Some("日志归档与滚动存储文件"),
                    input()
                        .value(move || log_dir.get())
                        .on_change(move |v: &str| log_dir.set(v.to_string()))
                        .w(px(160.0))
                        .h(px(24.0))
                        .rounded(px(4.0))
                        .border(px(1.0))
                        .border_color(input_border)
                        .bg(card_bg)
                        .text_xs()
                        .text_color(text_1),
                    theme,
                ),
                setting_row(
                    "输出至标准控制台",
                    Some("终端启动模式下打印实时日志流"),
                    render_toggle(
                        console_output.get(),
                        theme,
                        move |next| {
                            console_output.set(next);
                        },
                    ),
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
                        .child(sound_btn("经典滴答", 1, settings, accent, divider, text_1))
                        .child(sound_btn("清脆按键", 2, settings, accent, divider, text_1))
                        .child(sound_btn("柔和水滴", 3, settings, accent, divider, text_1)),
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
                            crate::services::copy_sound::play_copy_sound(settings.get().copy_sound_index);
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
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if current.get() == lvl { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            current.set(lvl.to_string());
        })
        .child(
            text(lvl)
                .text_color(move || if current.get() == lvl { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}

fn sound_btn(
    name: &'static str,
    idx: usize,
    settings: Signal<crate::core::settings::GuiSettings>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if settings.get().copy_sound_index == idx { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            settings.update(|s| s.copy_sound_index = idx);
            crate::services::copy_sound::play_copy_sound(idx);
        })
        .child(
            text(name)
                .text_color(move || if settings.get().copy_sound_index == idx { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}
