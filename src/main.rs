//! Binary entry point (`amemory`) acting as the pure composition root.

use eyre::{Result, eyre};
use toolkit_rs::{logger, painc::set_panic_handler};
use xcat::{AppConfig, run};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let config = AppConfig::load(None)?;
    set_panic_handler(config.to_toolkit_panic_conf());
    logger::setup(config.to_toolkit_log_config()).map_err(|e| eyre!("初始化日志组件失败: {e}"))?;
    run(config).await
}
