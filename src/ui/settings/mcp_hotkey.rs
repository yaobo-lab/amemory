//! Tab 4: MCP Protocol Server and Global Hotkey Settings.

use nsh_base::input;
use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::state::app::AppStore;
use crate::ui::components::toggle::render_toggle;
use crate::ui::theme::ClippiTheme;

/// Render Tab 4: MCP server protocol and shortcut configuration.
#[must_use]
pub fn render_mcp_hotkey_tab(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let settings = store.settings;
    let text_1 = theme.text_1;
    let card_bg = theme.card_bg;
    let input_border = theme.input_border;

    let mcp_port = signal("8000".to_string());
    let mcp_endpoint = signal("/mcp".to_string());
    let allow_remote = signal(false);

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(settings_group(
            "MCP 协议服务 (Model Context Protocol)",
            theme,
            vec![
                setting_row(
                    "HTTP/SSE 监听端口",
                    Some("供 Claude / Cursor / 终端连接的 MCP 端点"),
                    input()
                        .value(move || mcp_port.get())
                        .on_change(move |v: &str| mcp_port.set(v.to_string()))
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
                    "服务路由端点",
                    Some("MCP 握手及工具调用路由"),
                    input()
                        .value(move || mcp_endpoint.get())
                        .on_change(move |v: &str| mcp_endpoint.set(v.to_string()))
                        .w(px(120.0))
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
                    "允许远程局域网访问",
                    Some("开启后绑定 0.0.0.0，关闭仅允许 127.0.0.1"),
                    render_toggle(
                        allow_remote.get(),
                        theme,
                        move |next| {
                            allow_remote.set(next);
                        },
                    ),
                    theme,
                ),
            ],
        ))
        .child(settings_group(
            "全局呼出热键 (Global Summon Hotkey)",
            theme,
            vec![
                setting_row(
                    "主窗口呼出快捷键",
                    Some("按下直接唤起 amemory 面板并聚焦搜索框"),
                    view()
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(4.0))
                        .bg(theme.divider)
                        .child(
                            text(settings.get().global_summon_hotkey)
                                .text_color(text_1)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        ),
                    theme,
                ),
                setting_row(
                    "回车选中自动填入剪贴板",
                    Some("按回车键将知识卡片内容复制到系统剪贴板"),
                    render_toggle(
                        settings.get().paste_on_select,
                        theme,
                        move |next| {
                            settings.update(|s| s.paste_on_select = next);
                        },
                    ),
                    theme,
                ),
            ],
        ))
}
