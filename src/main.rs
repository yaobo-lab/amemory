//! Binary entry point (`amemory`) acting as the pure composition root.

use eyre::{eyre, Result};
use toolkit_rs::{logger, painc::set_panic_handler};
use xcat::{AppConfig, ServerMode, run};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut config = AppConfig::load(None)?;

    // 默认启动 GUI 模式，若命令行传入指定模式参数则按参数执行
    config.server.mode = ServerMode::Gui;

    let args: Vec<String> = std::env::args().collect();
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
    run(config).await
}
