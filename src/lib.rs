//! Memvid MCP 服务的核心模块。

#![expect(
    unused_crate_dependencies,
    reason = "color-eyre is used only by the package's binary entry point"
)]

/// 应用启动与 MCP 传输。
pub mod app;
mod books;
/// 应用配置的加载与验证。
pub mod config;
mod embedding;
mod models;
mod service;
mod skills;
