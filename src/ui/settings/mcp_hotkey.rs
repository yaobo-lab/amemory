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
    let accent = theme.accent;
    let divider = theme.divider;

    let cfg = store.app_config.get();
    let mcp_port = signal(cfg.server.port.to_string());
    let mcp_endpoint = signal(cfg.server.endpoint_path);
    let allow_remote = signal(cfg.server.allow_remote);

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
                        .on_change(move |v: &str| {
                            let val = v.to_string();
                            mcp_port.set(val.clone());
                            if let Ok(port) = val.trim().parse::<u16>()
                                && port > 0
                            {
                                store.update_app_config(move |c| {
                                    c.server.port = port;
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
                    "服务路由端点",
                    Some("MCP 握手及工具调用路由"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            input()
                                .value(move || mcp_endpoint.get())
                                .on_change(move |v: &str| {
                                    let val = v.to_string();
                                    mcp_endpoint.set(val.clone());
                                    if val.starts_with('/') {
                                        store.update_app_config(move |c| {
                                            c.server.endpoint_path = val;
                                        });
                                    }
                                })
                                .w(px(96.0))
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
                                .px(px(6.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .bg(accent)
                                .cursor_pointer()
                                .on_press(move || {
                                    let url = store.mcp_endpoint_url();
                                    crate::platform::clipboard::write_text(&url);
                                    store.show_toast(format!("已复制 MCP 地址: {url}"));
                                })
                                .child(
                                    text("复制")
                                        .text_color(rgb(0xffffff))
                                        .font_weight(FontWeight::BOLD)
                                        .text_xs(),
                                ),
                        ),
                    theme,
                ),
                setting_row(
                    "允许远程局域网访问",
                    Some("开启后绑定 0.0.0.0，关闭仅允许 127.0.0.1"),
                    render_toggle(allow_remote.get(), theme, move |next| {
                        allow_remote.set(next);
                        store.update_app_config(move |c| {
                            c.server.allow_remote = next;
                        });
                    }),
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
                        .flex_row()
                        .gap(px(4.0))
                        .child(hotkey_chip("Alt+Space", store, accent, divider, text_1))
                        .child(hotkey_chip(
                            "Ctrl+Shift+Space",
                            store,
                            accent,
                            divider,
                            text_1,
                        ))
                        .child(hotkey_chip("Alt+M", store, accent, divider, text_1)),
                    theme,
                ),
                setting_row(
                    "回车选中自动填入剪贴板",
                    Some("按回车键将知识卡片内容复制到系统剪贴板"),
                    render_toggle(settings.get().paste_on_select, theme, move |next| {
                        store.update_settings(|s| s.paste_on_select = next);
                    }),
                    theme,
                ),
            ],
        ))
}

fn hotkey_chip(
    hotkey: &'static str,
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
            if settings.get().global_summon_hotkey == hotkey {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            store.update_settings(|s| s.global_summon_hotkey = hotkey.to_string());
            crate::platform::hotkey::update_global_hotkey(hotkey);
            store.show_toast(format!("已更新全局热键: {hotkey}"));
        })
        .child(
            text(hotkey)
                .text_color(move || {
                    if settings.get().global_summon_hotkey == hotkey {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .font_weight(FontWeight::BOLD)
                .text_xs(),
        )
}
