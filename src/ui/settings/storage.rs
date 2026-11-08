//! Tab 2: Storage and Knowledge Catalog Settings (Skills / Books paths, limits).

use std::path::PathBuf;

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::assets::glyphs;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 2: Storage paths and boundary settings.
#[must_use]
pub fn render_storage_tab(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let divider = theme.divider;
    let accent = theme.accent;

    let cfg = store.app_config.get();
    let skills_path = signal(cfg.storage.skills.display().to_string());
    let books_path = signal(cfg.storage.books.display().to_string());
    let max_size_mb = signal(
        (cfg.document_import.max_file_bytes / (1024 * 1024))
            .max(1)
            .to_string(),
    );

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
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            input()
                                .value(move || skills_path.get())
                                .on_change(move |v: &str| {
                                    let val = v.to_string();
                                    skills_path.set(val.clone());
                                    if !val.trim().is_empty() {
                                        store.update_app_config(move |c| {
                                            c.storage.skills = PathBuf::from(&val);
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
                                    let path = PathBuf::from(skills_path.get());
                                    crate::services::amemory_runtime::open_path_in_explorer(&path);
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
                    "Books 目录",
                    Some("存储知识书本与分段知识库的根目录"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            input()
                                .value(move || books_path.get())
                                .on_change(move |v: &str| {
                                    let val = v.to_string();
                                    books_path.set(val.clone());
                                    if !val.trim().is_empty() {
                                        store.update_app_config(move |c| {
                                            c.storage.books = PathBuf::from(&val);
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
                                    let path = PathBuf::from(books_path.get());
                                    crate::services::amemory_runtime::open_path_in_explorer(&path);
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
                    "实时监听文件变更",
                    Some("当外部编辑器修改技能或书本时自动刷新"),
                    render_toggle(
                        store.settings.get().auto_watch_catalog,
                        theme,
                        move |next| {
                            store.update_settings(|s| s.auto_watch_catalog = next);
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
                        .on_change(move |v: &str| {
                            let val = v.to_string();
                            max_size_mb.set(val.clone());
                            if let Ok(mb) = val.trim().parse::<usize>()
                                && mb > 0
                            {
                                store.update_app_config(move |c| {
                                    c.document_import.max_file_bytes = mb * 1024 * 1024;
                                });
                            }
                        })
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
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text("已强制启用")
                                .text_color(accent)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        )
                        .child(
                            view()
                                .px(px(8.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .bg(accent)
                                .cursor_pointer()
                                .on_press(move || {
                                    store.update_app_config(|_| {});
                                    store.show_toast("已保存存储配置到 etc/config.toml");
                                })
                                .child(
                                    text("保存配置")
                                        .text_color(rgb(0xffffff))
                                        .font_weight(FontWeight::BOLD)
                                        .text_xs(),
                                ),
                        ),
                    theme,
                ),
            ],
        ))
}
