//! Tab 2: Storage and Knowledge Catalog Settings (Skills / Books paths, limits).

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 2: Storage paths and boundary settings.
pub fn render_storage_tab(_store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let _divider = theme.divider;

    let skills_path = signal("data/skills".to_string());
    let books_path = signal("data/books".to_string());
    let max_size_mb = signal("10".to_string());
    let auto_watch = signal(true);

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(settings_group(
            "知识库目录配置 (Knowledge Repositories)",
            theme,
            vec![
                setting_row(
                    "Skills 目录",
                    Some("存储 Prompt 与 Agent 技能文件的根目录"),
                    input()
                        .value(move || skills_path.get())
                        .on_change(move |v: &str| skills_path.set(v.to_string()))
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
                    "Books 目录",
                    Some("存储知识书本与分段知识库的根目录"),
                    input()
                        .value(move || books_path.get())
                        .on_change(move |v: &str| books_path.set(v.to_string()))
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
                    "实时监听文件变更",
                    Some("当外部编辑器修改技能或书本时自动刷新"),
                    render_toggle(
                        auto_watch.get(),
                        theme,
                        move |next| {
                            auto_watch.set(next);
                        },
                    ),
                    theme,
                ),
            ],
        ))
        .child(settings_group(
            "存储与导入限制 (Limits & Boundaries)",
            theme,
            vec![
                setting_row(
                    "单文档最大体积 (MB)",
                    Some("超过此大小的文档将在导入时分块或拦截"),
                    input()
                        .value(move || max_size_mb.get())
                        .on_change(move |v: &str| max_size_mb.set(v.to_string()))
                        .w(px(80.0))
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
                    "存储根路径校验",
                    Some("禁止跨越项目工作目录外的恶意路径访问"),
                    text("已强制启用")
                        .text_color(theme.accent)
                        .font_weight(FontWeight::BOLD)
                        .text_xs(),
                    theme,
                ),
            ],
        ))
}
