//! Tab 6: About & Version Update — 1:1 replica of Clippi update mechanism.

use nsh_core::prelude::*;

use super::{setting_row, settings_group};
use crate::core::i18n_keys::I18nKey;
use crate::services::update::UpdatePhase;
use crate::state::app::AppStore;
use crate::ui::theme::ClippiTheme;

/// Render Tab 6: About & version update with progress bar.
pub fn render_version_tab(
    store: AppStore,
    theme: ClippiTheme,
    on_check_update: impl Fn() + 'static,
    on_start_download: impl Fn() + 'static,
    on_restart_update: impl Fn() + 'static,
) -> impl IntoElement {
    let settings = store.settings;
    let phase = store.update_phase;
    let update_available = store.update_available;
    let stats = store.stats;

    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let danger = theme.danger;

    let current_version = env!("CARGO_PKG_VERSION");

    view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        // ── Group 1: Version & Update Card ──
        .child(
            view()
                .w_full()
                .bg(card_bg)
                .rounded(px(8.0))
                .border(px(1.0))
                .border_color(divider)
                .p(px(12.0))
                .flex_col()
                .gap(px(8.0))
                // Version title
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(text("🧠").text_sm())
                                .child(
                                    text(format!("amemory v{}", current_version))
                                        .text_color(text_1)
                                        .font_weight(FontWeight::BOLD)
                                        .text_xs(),
                                ),
                        )
                        .child(
                            text("已是最新稳定版")
                                .text_color(accent)
                                .text_xs(),
                        ),
                )
                // Status Description
                .child(
                    text(move || match phase.get() {
                        UpdatePhase::Idle => "当前为最新版，随时可手动检查更新".to_string(),
                        UpdatePhase::Checking => "正在连接 GitHub Releases 检查新版本...".to_string(),
                        UpdatePhase::UpToDate => "太棒了！当前已是最新版本。".to_string(),
                        UpdatePhase::Available(v) => format!("发现新版本 v{}，点击立即下载更新", v.latest_version),
                        UpdatePhase::UpdateAvailable => "发现新版本，点击立即检查".to_string(),
                        UpdatePhase::Downloading { progress } => {
                            format!("正在下载更新安装包... {:.1}%", progress * 100.0)
                        }
                        UpdatePhase::Verifying => "正在进行 SHA256 完整性校验...".to_string(),
                        UpdatePhase::Installing => "正在准备安装更新...".to_string(),
                        UpdatePhase::ReadyToRestart => "新版本已就绪，点击重启应用即可完成升级".to_string(),
                        UpdatePhase::Error(e) => format!("检查或更新失败: {}", e),
                    })
                    .text_color(move || match phase.get() {
                        UpdatePhase::Error(_) => danger,
                        _ => text_3,
                    })
                    .text_xs(),
                )
                // Progress Bar (shown when Downloading)
                .child(
                    when(
                        move || matches!(phase.get(), UpdatePhase::Downloading { .. }),
                        move || {
                            let p = match phase.get() {
                                UpdatePhase::Downloading { progress } => progress,
                                _ => 0.0,
                            };
                            view()
                                .w_full()
                                .h(px(6.0))
                                .rounded(px(3.0))
                                .bg(divider)
                                .child(
                                    view()
                                        .w(px(260.0 * p.clamp(0.0, 1.0)))
                                        .h_full()
                                        .rounded(px(3.0))
                                        .bg(accent),
                                )
                        },
                    ),
                )
                // Action Row: Auto-check toggle & Button
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .pt(px(4.0))
                        // Auto check checkbox
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .cursor_pointer()
                                .on_press(move || {
                                    settings.update(|s| s.auto_check_updates = !s.auto_check_updates);
                                })
                                .child(
                                    text(move || if settings.get().auto_check_updates { "☑" } else { "☐" })
                                        .text_color(move || if settings.get().auto_check_updates { accent } else { text_3 })
                                        .text_xs(),
                                )
                                .child(
                                    text(I18nKey::SettingAutoCheckUpdate.text())
                                        .text_color(text_2)
                                        .text_xs(),
                                ),
                        )
                        // Action button
                        .child(
                            view()
                                .px(px(12.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .bg(move || match phase.get() {
                                    UpdatePhase::Checking | UpdatePhase::Downloading { .. } | UpdatePhase::Verifying => divider,
                                    _ => accent,
                                })
                                .cursor_pointer()
                                .on_press(move || match phase.get() {
                                    UpdatePhase::Idle | UpdatePhase::UpToDate => on_check_update(),
                                    UpdatePhase::Available(_) => on_start_download(),
                                    UpdatePhase::ReadyToRestart => on_restart_update(),
                                    UpdatePhase::Error(_) => on_check_update(),
                                    _ => {}
                                })
                                .child(
                                    text(move || match phase.get() {
                                        UpdatePhase::Idle => "检查更新".to_string(),
                                        UpdatePhase::Checking => "正在检查...".to_string(),
                                        UpdatePhase::UpToDate => "检查更新".to_string(),
                                        UpdatePhase::Available(_) | UpdatePhase::UpdateAvailable => "立即更新".to_string(),
                                        UpdatePhase::Downloading { .. } => "下载中...".to_string(),
                                        UpdatePhase::Verifying => "校验中...".to_string(),
                                        UpdatePhase::Installing => "准备中...".to_string(),
                                        UpdatePhase::ReadyToRestart => "重启应用".to_string(),
                                        UpdatePhase::Error(_) => "重试".to_string(),
                                    })
                                    .text_color(rgb(0xffffff))
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                                ),
                        ),
                )
                // Release notes (when available)
                .child(
                    when(
                        move || update_available.get().is_some(),
                        move || {
                            let note = update_available
                                .get()
                                .map(|u| u.release_notes)
                                .unwrap_or_default();
                            view()
                                .w_full()
                                .p(px(8.0))
                                .rounded(px(4.0))
                                .bg(divider)
                                .child(
                                    text(format!("更新说明:\n{}", note))
                                        .text_color(text_2)
                                        .text_xs(),
                                )
                        },
                    ),
                ),
        )
        // ── Group 2: System & Engine Runtime Status ──
        .child(settings_group(
            "系统与运行时环境 (Runtime Environment)",
            theme,
            vec![
                setting_row(
                    "XERJ 搜索引擎状态",
                    None,
                    text(move || {
                        let snap = stats.get();
                        format!("已连接 ({} 段 / {} 篇文档 / {:.1}ms)", snap.segments, snap.doc_count, snap.p95_latency_ms)
                    })
                    .text_color(accent)
                    .text_xs(),
                    theme,
                ),
                setting_row(
                    "MCP 协议代理服务",
                    None,
                    text("运行中 (端口: 8000)")
                        .text_color(accent)
                        .text_xs(),
                    theme,
                ),
                setting_row(
                    "底层操作系统",
                    None,
                    text("Windows x86_64 (DirectComposition)")
                        .text_color(text_3)
                        .text_xs(),
                    theme,
                ),
            ],
        ))
}
