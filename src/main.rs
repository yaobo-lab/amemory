//! Binary entry point (`amemory`) acting as the pure composition root.

use eyre::{eyre, Result};
use toolkit_rs::{logger, painc::set_panic_handler};
use xcat::{AppConfig, ServerMode, run};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut config = AppConfig::load(None)?;

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--gui" || a == "-g" || a == "gui") {
        config.server.mode = ServerMode::Gui;
    }

    set_panic_handler(config.to_toolkit_panic_conf());
    logger::setup(config.to_toolkit_log_config()).map_err(|e| eyre!("初始化日志组件失败: {e}"))?;
    run(config).await
}
