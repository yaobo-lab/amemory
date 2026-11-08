//! Binary entry point (`amemory`) acting as the pure composition root.

use eyre::{Result, eyre};
use toolkit_rs::{logger, painc::set_panic_handler};
use xcat::{AppConfig, ServerMode, run};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut config = AppConfig::load(None)?;

    // 默认启动 GUI 模式，若命令行传入指定模式参数则按参数执行
    config.server.mode = ServerMode::Gui;

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Amemory (xcat) - 桌面知识与记忆中心 / MCP 智能代理服务\n");
        println!("用法: xcat [OPTIONS]\n");
        println!("选项:");
        println!("  -g, --gui        启动原生桌面 GUI 主程序 (默认)");
        println!("  --mcp-http       启动 MCP HTTP 协议服务");
        println!("  --mcp-stdio      启动 MCP Stdio 管道服务");
        println!("  --server         启动完整 HTTP API 服务");
        println!("  --sync           执行一次性数据全量同步并退出");
        println!("  -h, --help       显示帮助信息");
        return Ok(());
    }

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--mcp-http" | "mcp-http" => config.server.mode = ServerMode::McpHttp,
            "--mcp-stdio" | "mcp-stdio" => config.server.mode = ServerMode::McpStdio,
            "--server" | "server" => config.server.mode = ServerMode::Server,
            "--sync" | "sync" => config.server.mode = ServerMode::Sync,
            "--gui" | "-g" | "gui" => config.server.mode = ServerMode::Gui,
            _ => {}
        }
    }

    set_panic_handler(config.to_toolkit_panic_conf());
    logger::setup(config.to_toolkit_log_config()).map_err(|e| eyre!("初始化日志组件失败: {e}"))?;

    // 在 GUI 桌面模式下执行单实例防重守护
    if config.server.mode == ServerMode::Gui && !ensure_single_instance(config.server.port) {
        xcat::platform::hotkey::summon_app_window();
        tracing::info!("amemory 实例已在后台运行，已激活现有主窗口。");
        return Ok(());
    }

    run(config).await
}

fn ensure_single_instance(port: u16) -> bool {
    let lock_port = if port > 0 {
        port.saturating_add(10000)
    } else {
        19876
    };
    std::net::TcpListener::bind(format!("127.0.0.1:{lock_port}")).is_ok_and(|listener| {
        Box::leak(Box::new(listener));
        true
    })
}
