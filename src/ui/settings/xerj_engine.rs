//! Tab 3: XERJ Search Engine Settings (Data dir, embed mode, index names, rebuild).

use std::path::PathBuf;

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::assets::glyphs;
use crate::ui::theme::ClippiTheme;

/// Render Tab 3: XERJ search engine configuration and index maintenance.
pub fn render_xerj_tab(
    store: AppStore,
    theme: ClippiTheme,
    on_rebuild_index: impl Fn() + 'static,
) -> impl IntoElement {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let accent = theme.accent;
    let divider = theme.divider;

    let cfg = store.app_config.get();
    let xerj_data_dir = signal(cfg.xerj.data_dir.display().to_string());
    let embed_mode = signal(cfg.xerj.embed_mode);
    let rest_port = signal(cfg.xerj.es_compat_port.to_string());

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(settings_group(
            "XERJ 核心引擎参数 (XERJ Core Engine)",
            theme,
            vec![
                setting_row(
                    "索引持久化路径",
                    Some("倒排索引与向量段存储目录"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            input()
                                .value(move || xerj_data_dir.get())
                                .on_change(move |v: &str| {
                                    let val = v.to_string();
                                    xerj_data_dir.set(val.clone());
                                    if !val.trim().is_empty() {
                                        store.update_app_config(move |c| {
                                            c.xerj.data_dir = PathBuf::from(&val);
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
                                    let path = PathBuf::from(xerj_data_dir.get());
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
                    "向量嵌入模型 (Embedding)",
                    Some("语义搜索向量特征提取算法"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .bg(move || {
                                    if embed_mode.get() == "lexical" {
                                        accent
                                    } else {
                                        divider
                                    }
                                })
                                .cursor_pointer()
                                .on_press(move || {
                                    embed_mode.set("lexical".to_string());
                                    store.update_app_config(|c| {
                                        c.xerj.embed_mode = "lexical".to_string();
                                    });
                                    store.show_toast("已切换为 lexical 词法引擎模式");
                                })
                                .child(
                                    text("lexical")
                                        .text_color(move || {
                                            if embed_mode.get() == "lexical" {
                                                rgb(0xffffff)
                                            } else {
                                                text_1
                                            }
                                        })
                                        .text_xs(),
                                ),
                        )
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .bg(move || {
                                    if embed_mode.get() == "bge-small-zh-v1.5" {
                                        accent
                                    } else {
                                        divider
                                    }
                                })
                                .cursor_pointer()
                                .on_press(move || {
                                    embed_mode.set("bge-small-zh-v1.5".to_string());
                                    store.update_app_config(|c| {
                                        c.xerj.embed_mode = "bge-small-zh-v1.5".to_string();
                                    });
                                    store.show_toast("已切换为 bge-small-zh-v1.5 向量引擎模式");
                                })
                                .child(
                                    text("bge-small-zh")
                                        .text_color(move || {
                                            if embed_mode.get() == "bge-small-zh-v1.5" {
                                                rgb(0xffffff)
                                            } else {
                                                text_1
                                            }
                                        })
                                        .text_xs(),
                                ),
                        ),
                    theme,
                ),
                setting_row(
                    "ES 兼容 REST 端口",
                    Some("提供标准 Elasticsearch HTTP API 访问端口"),
                    input()
                        .value(move || rest_port.get())
                        .on_change(move |v: &str| {
                            let val = v.to_string();
                            rest_port.set(val.clone());
                            if let Ok(port) = val.trim().parse::<u16>()
                                && port > 0
                            {
                                store.update_app_config(move |c| {
                                    c.xerj.es_compat_port = port;
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
            ],
        ))
        .child(settings_group(
            "索引维护与优化 (Index Maintenance)",
            theme,
            vec![setting_row(
                "重新构建索引",
                Some("全量扫描 Skills、Books 与 Memory 重新生成 BM25 与向量索引"),
                view()
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(5.0))
                    .bg(accent)
                    .cursor_pointer()
                    .on_press(move || {
                        on_rebuild_index();
                    })
                    .child(
                        text("立即重构")
                            .text_color(rgb(0xffffff))
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                    ),
                theme,
            )],
        ))
}
