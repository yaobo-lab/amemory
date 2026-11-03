//! 加载并验证 `etc/config.toml` 中的应用配置。

use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;
use toolkit_rs::{config, logger::LogConfig};

/// 应用配置文件的固定路径。
pub const CONFIG_PATH: &str = "etc/config.toml";

/// Memvid MCP 的完整运行配置。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// MCP Server 和传输配置。
    pub server: ServerConfig,
    /// Memvid 与 Skill 路径配置。
    pub storage: StorageConfig,
    /// 日志输出与轮转配置。
    pub logger: LogConfig,
    /// panic 处理配置。
    pub panic: PanicConfig,
}

/// MCP Server 的传输配置。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// MCP 传输方式。
    pub transport: TransportMode,
    /// Streamable HTTP 的监听地址。
    pub listen: SocketAddr,
    /// 是否仅同步 Skill 后退出。
    pub sync_skills_only: bool,
}

/// Memvid 存储和 Skill 源目录配置。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    /// Memvid 存储文件路径。
    pub memory: PathBuf,
    /// Skill 根目录路径。
    pub skills: PathBuf,
    /// Markdown 电子书根目录路径。
    pub books: PathBuf,
}

/// panic hook 的行为配置。
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanicConfig {
    /// 记录 panic 后是否立即退出进程。
    pub exit: bool,
}

/// MCP Server 支持的传输方式。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    /// 通过标准输入输出与本地 MCP Host 通信。
    Stdio,
    /// 通过 Stateless Streamable HTTP 通信。
    Http,
}

/// 从 [`CONFIG_PATH`] 读取并验证配置。
///
/// # Errors
///
/// 配置文件不存在、TOML 内容无法反序列化，或配置值违反运行约束时返回错误。
pub fn load() -> eyre::Result<AppConfig> {
    let config = config::read_config::<AppConfig>(CONFIG_PATH)
        .map_err(|error| eyre::eyre!("failed to read {CONFIG_PATH}: {error:#}"))?;
    config.validate()?;
    Ok(config)
}

impl AppConfig {
    fn validate(&self) -> eyre::Result<()> {
        if self.storage.memory.as_os_str().is_empty() {
            eyre::bail!("storage.memory must not be empty");
        }
        if self.storage.skills.as_os_str().is_empty() {
            eyre::bail!("storage.skills must not be empty");
        }
        if self.storage.books.as_os_str().is_empty() {
            eyre::bail!("storage.books must not be empty");
        }
        if self.storage.books == self.storage.skills {
            eyre::bail!("storage.books and storage.skills must use different directories");
        }
        if self.logger.size_mb == 0 {
            eyre::bail!("logger.size_mb must be at least 1");
        }
        if self.server.transport == TransportMode::Http && !self.server.listen.ip().is_loopback() {
            eyre::bail!(
                "server.listen must be a loopback address; use a local reverse proxy for remote access"
            );
        }
        if self.server.transport == TransportMode::Stdio && self.logger.console {
            eyre::bail!(
                "logger.console must be false for stdio because stdout carries the MCP protocol"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_config(transport: TransportMode) -> AppConfig {
        let logger = LogConfig {
            console: transport == TransportMode::Http,
            ..LogConfig::default()
        };
        AppConfig {
            server: ServerConfig {
                transport,
                listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000),
                sync_skills_only: false,
            },
            storage: StorageConfig {
                memory: PathBuf::from("memvid.mv2"),
                skills: PathBuf::from("skills"),
                books: PathBuf::from("books"),
            },
            logger,
            panic: PanicConfig { exit: true },
        }
    }

    #[test]
    fn loads_repository_configuration() {
        let config = load().expect("repository configuration should be valid");

        assert_eq!(config.server.transport, TransportMode::Http);
        assert_eq!(config.server.listen.port(), 3000);
        assert_eq!(config.storage.memory, PathBuf::from("memvid.mv2"));
        assert_eq!(config.storage.skills, PathBuf::from("skills"));
        assert_eq!(config.storage.books, PathBuf::from("books"));
    }

    #[test]
    fn accepts_http_configuration() {
        assert!(
            test_config(TransportMode::Http).validate().is_ok(),
            "valid HTTP configuration should pass"
        );
    }

    #[test]
    fn rejects_public_http_listener() {
        let mut config = test_config(TransportMode::Http);
        config.server.listen = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3000);

        let error = config
            .validate()
            .expect_err("public HTTP listener should fail");
        assert!(
            error.to_string().contains("loopback"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_console_logging_for_stdio() {
        let mut config = test_config(TransportMode::Stdio);
        config.logger.console = true;

        let error = config
            .validate()
            .expect_err("stdio console logging should fail");
        assert!(
            error
                .to_string()
                .contains("stdout carries the MCP protocol"),
            "unexpected error: {error}"
        );
    }
}
