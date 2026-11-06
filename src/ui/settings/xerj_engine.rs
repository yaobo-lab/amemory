//! Tab 3: XERJ Search Engine Settings (Data dir, embed mode, index names, rebuild).

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::theme::ClippiTheme;

pub fn render_xerj_tab(
    _store: AppStore,
    theme: ClippiTheme,
    on_rebuild_index: impl Fn() + 'static,
) -> impl IntoElement {
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;
    let accent = theme.accent;

    let xerj_data_dir = signal("data/xerj".to_string());
    let embed_mode = signal("bge-small-zh-v1.5".to_string());
    let rest_port = signal("9200".to_string());

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
                    input()
                        .value(move || xerj_data_dir.get())
                        .on_change(move |v: &str| xerj_data_dir.set(v.to_string()))
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
                    "向量嵌入模型 (Embedding)",
                    Some("语义搜索向量特征提取算法"),
                    input()
                        .value(move || embed_mode.get())
                        .on_change(move |v: &str| embed_mode.set(v.to_string()))
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
                    "ES 兼容 REST 端口",
                    Some("提供标准 Elasticsearch HTTP API 访问端口"),
                    input()
                        .value(move || rest_port.get())
                        .on_change(move |v: &str| rest_port.set(v.to_string()))
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
            vec![
                setting_row(
                    "重新构建索引",
                    Some("全量扫描 Skills 和 Books 重新生成 BM25 与向量索引"),
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
                ),
            ],
        ))
}
