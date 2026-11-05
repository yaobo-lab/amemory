//! 通过标准输入输出或 Streamable HTTP 运行 Memvid 模型上下文协议服务器。

#![expect(
    unused_crate_dependencies,
    reason = "the binary delegates application modules and their dependencies to the library crate"
)]

use toolkit_rs::{
    logger,
    painc::{PaincConf, set_panic_handler},
};
use xflash_mcp::{app, config};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let config = config::load()?;
    set_panic_handler(PaincConf {
        version: env!("CARGO_PKG_VERSION").into(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").into(),
        painc_exit: config.panic.exit,
    });
    logger::setup(config.logger.clone())
        .map_err(|error| eyre::eyre!("failed to initialize logger: {error:#}"))?;
    log::info!(
        "server starting: crate={}, version={}, config={}",
        env!("CARGO_CRATE_NAME"),
        env!("CARGO_PKG_VERSION"),
        config::CONFIG_PATH
    );
    app::run(config).await
}
