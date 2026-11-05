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
    /// 电子书二进制文档导入配置。
    pub document_import: DocumentImportConfig,
    /// 文档向量嵌入配置。
    pub embedding: EmbeddingConfig,
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

/// Book 文档上传和本地读取的安全边界。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentImportConfig {
    /// 单个 PDF 或 Office 文档允许的最大字节数。
    pub max_file_bytes: usize,
    /// `source_path` 允许读取的根目录；Base64 上传不受此项影响。
    pub allowed_roots: Vec<PathBuf>,
}

/// 导入文档时使用的向量嵌入配置。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConfig {
    /// 嵌入提供方。
    pub provider: EmbeddingProviderMode,
    /// 本地 ONNX 文本模型名。
    pub local_model: String,
    /// `OpenAI` Embeddings 模型名。
    pub openai_model: String,
    /// 本地 ONNX 模型和 tokenizer 所在目录。
    pub models_dir: PathBuf,

    /// `OpenAI` API key 所在环境变量名。
    pub api_key_env: String,
    /// `OpenAI` 兼容 API 的基础 URL。
    pub base_url: String,
    /// `OpenAI` 请求超时秒数。
    pub timeout_secs: u64,
    /// `OpenAI` 限流请求最大重试次数。
    pub max_retries: u32,
}

/// 文档导入支持的向量嵌入提供方。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProviderMode {
    /// 只建立 Memvid 词法和时间索引。
    None,
    /// 使用本地 ONNX 文本模型。
    Local,
    /// 使用 `OpenAI` 兼容 Embeddings API。
    Openai,
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
        if self.document_import.max_file_bytes == 0 {
            eyre::bail!("document_import.max_file_bytes must be at least 1");
        }
        if self
            .document_import
            .allowed_roots
            .iter()
            .any(|root| root.as_os_str().is_empty())
        {
            eyre::bail!("document_import.allowed_roots must not contain empty paths");
        }
        if self.embedding.provider == EmbeddingProviderMode::Local {
            if self.embedding.local_model.trim().is_empty() {
                eyre::bail!("embedding.local_model must not be empty for local embeddings");
            }
            if self.embedding.models_dir.as_os_str().is_empty() {
                eyre::bail!("embedding.models_dir must not be empty for local embeddings");
            }
        }
        if self.embedding.provider == EmbeddingProviderMode::Openai {
            if self.embedding.openai_model.trim().is_empty() {
                eyre::bail!("embedding.openai_model must not be empty for OpenAI embeddings");
            }
            if self.embedding.api_key_env.trim().is_empty() {
                eyre::bail!("embedding.api_key_env must not be empty for OpenAI embeddings");
            }
            if self.embedding.base_url.trim().is_empty() {
                eyre::bail!("embedding.base_url must not be empty for OpenAI embeddings");
            }
            if self.embedding.timeout_secs == 0 {
                eyre::bail!("embedding.timeout_secs must be at least 1");
            }
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
            document_import: DocumentImportConfig {
                max_file_bytes: 32 * 1024 * 1024,
                allowed_roots: vec![PathBuf::from("uploads")],
            },
            embedding: EmbeddingConfig {
                provider: EmbeddingProviderMode::None,
                local_model: "bge-small-en-v1.5".to_owned(),
                openai_model: "text-embedding-3-small".to_owned(),
                models_dir: PathBuf::from("models/text"),

                api_key_env: "OPENAI_API_KEY".to_owned(),
                base_url: "https://api.openai.com/v1".to_owned(),
                timeout_secs: 30,
                max_retries: 3,
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
        assert_eq!(config.document_import.max_file_bytes, 32 * 1024 * 1024);
        assert_eq!(config.embedding.provider, EmbeddingProviderMode::None);
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
