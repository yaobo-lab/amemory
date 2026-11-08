//! Application configuration models and validation for Amemory.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use adapter::{DEFAULT_BOOKS_INDEX, DEFAULT_MEMORY_INDEX, DEFAULT_SKILLS_INDEX, XerjConfig};
use eyre::{Result, bail, eyre};
use serde::{Deserialize, Serialize};
use toolkit_rs::{
    config::read_config_default,
    logger::{LogConfig, LogStyle, string_to_level},
    painc::PaincConf,
};
use xcat_mcp::DocumentImportPolicy;

/// Default configuration file path relative to the workspace root.
pub const DEFAULT_CONFIG_PATH: &str = "etc/config.toml";

/// Top-level runtime configuration loaded from `etc/config.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server mode and MCP HTTP transport settings.
    #[serde(default)]
    pub server: ServerConfig,
    /// Filesystem paths for Skill and Book catalogs.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Embedded XERJ engine and listener configuration.
    #[serde(default)]
    pub xerj: XerjSectionConfig,
    /// Security policy for local and Base64 document imports.
    #[serde(default)]
    pub document_import: DocumentImportConfig,
    /// Logger configuration (`toolkit-rs`).
    #[serde(default)]
    pub logger: LoggerSectionConfig,
    /// Panic handler configuration (`toolkit-rs`).
    #[serde(default)]
    pub panic: PanicSectionConfig,
}

impl AppConfig {
    /// Load [`AppConfig`] from `path` (defaulting to [`DEFAULT_CONFIG_PATH`]),
    /// or return [`AppConfig::default()`] if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file exists but cannot be parsed.
    pub fn load(path: Option<&str>) -> Result<Self> {
        let cfg_file = path.unwrap_or(DEFAULT_CONFIG_PATH);
        if !Path::new(cfg_file).is_file() {
            return Ok(Self::default());
        }
        read_config_default(cfg_file).map_err(|e| eyre!("加载配置文件 `{cfg_file}` 失败: {e}"))
    }

    /// Persist [`AppConfig`] to `path` (defaulting to [`DEFAULT_CONFIG_PATH`]) in TOML format.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file writing fails.
    pub fn save(&self, path: Option<&str>) -> Result<()> {
        let cfg_file = path.unwrap_or(DEFAULT_CONFIG_PATH);
        let full_path = crate::core::paths::resolve_relative(Path::new(cfg_file));
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| eyre!("创建配置目录 `{}` 失败: {e}", parent.display()))?;
        }
        let toml_str =
            toml::to_string_pretty(self).map_err(|e| eyre!("序列化配置为 TOML 失败: {e}"))?;
        std::fs::write(&full_path, toml_str)
            .map_err(|e| eyre!("写入配置文件 `{}` 失败: {e}", full_path.display()))
    }

    /// Validate configuration invariants and convert into runtime settings.
    ///
    /// # Errors
    ///
    /// Returns an error if paths, ports, or limits are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.storage.skills.as_os_str().is_empty() {
            bail!("storage.skills 路径不能为空");
        }
        if self.storage.books.as_os_str().is_empty() {
            bail!("storage.books 路径不能为空");
        }
        if self.xerj.data_dir.as_os_str().is_empty() {
            bail!("xerj.data_dir 路径不能为空");
        }
        if self.xerj.skills_index.trim().is_empty()
            || self.xerj.books_index.trim().is_empty()
            || self.xerj.memory_index.trim().is_empty()
        {
            bail!("xerj 索引名称 (skills_index / books_index / memory_index) 不能为空");
        }
        if self.document_import.max_file_bytes == 0 {
            bail!("document_import.max_file_bytes 必须大于 0");
        }
        if self.server.max_request_body_bytes == 0 {
            bail!("server.max_request_body_bytes 必须大于 0");
        }
        if !self.server.endpoint_path.starts_with('/') {
            bail!(
                "server.endpoint_path `{}` 必须以 `/` 开头",
                self.server.endpoint_path
            );
        }
        let bind_addr = self.server.socket_addr()?;
        if matches!(self.server.mode, ServerMode::McpHttp | ServerMode::Server)
            && !self.server.allow_remote
            && !bind_addr.ip().is_loopback()
        {
            bail!("非回环地址 `{bind_addr}` 需要显式设置 server.allow_remote = true");
        }
        Ok(())
    }

    /// Build the [`XerjConfig`] consumed by [`adapter::XerjAdapter`].
    #[must_use]
    pub fn to_xerj_config(&self) -> XerjConfig {
        XerjConfig {
            data_dir: self.xerj.data_dir.clone(),
            skills_index: self.xerj.skills_index.clone(),
            books_index: self.xerj.books_index.clone(),
            memory_index: self.xerj.memory_index.clone(),
            embed_mode: self.xerj.embed_mode.clone(),
            bind_address: self.xerj.bind_address.clone(),
            es_compat_port: self.xerj.es_compat_port,
            rest_port: self.xerj.rest_port,
            grpc_port: self.xerj.grpc_port,
            external_es_url: self.xerj.external_es_url.clone(),
        }
    }

    /// Resolve the optional XERJ Hub API key from `xerj.hub_api_key` or the
    /// `XERJ_HUB_API_KEY` environment variable.
    #[must_use]
    pub fn resolve_hub_api_key(&self) -> Option<String> {
        self.xerj
            .hub_api_key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                std::env::var("XERJ_HUB_API_KEY")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    }

    /// Build the [`DocumentImportPolicy`] consumed by [`xcat_mcp::AmemoryToolProvider`].
    #[must_use]
    pub fn to_import_policy(&self) -> DocumentImportPolicy {
        DocumentImportPolicy {
            allowed_roots: self.document_import.allowed_roots.clone(),
            max_file_bytes: self.document_import.max_file_bytes,
        }
    }

    /// Convert `[logger]` configuration into `toolkit_rs::logger::LogConfig`.
    ///
    /// In [`ServerMode::McpStdio`] mode, console output is forced off so `stdout`
    /// remains exclusively reserved for the MCP JSON-RPC protocol stream.
    #[must_use]
    pub fn to_toolkit_log_config(&self) -> LogConfig {
        let console = if self.server.mode == ServerMode::McpStdio {
            false
        } else {
            self.logger.console
        };
        LogConfig {
            level: string_to_level(&self.logger.level),
            size_mb: self.logger.size_mb,
            console,
            dir: self.logger.dir.clone(),
            keep_day: None,
            keep_file_count: Some(3),
            filters: None,
            log_name: Some("amemory".to_string()),
            style: LogStyle::Default,
            cleanup_sync: Some(false),
        }
    }

    /// Convert `[panic]` configuration into `toolkit_rs::painc::PaincConf`.
    #[must_use]
    pub fn to_toolkit_panic_conf(&self) -> PaincConf {
        PaincConf {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: "2026-11-06".to_string(),
            painc_exit: self.panic.exit_on_panic,
        }
    }
}

/// Execution mode for the `amemory` binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServerMode {
    /// Synchronize Skills and Books into XERJ and exit.
    Sync,
    /// Run unified MCP server over stdio.
    #[serde(alias = "stdio", alias = "mcp_stdio")]
    McpStdio,
    /// Run unified MCP server over Streamable HTTP.
    #[default]
    #[serde(alias = "http", alias = "mcp_http")]
    McpHttp,
    /// Run full XERJ server runtime (REST + ES-compat + Console + gRPC) alongside unified MCP HTTP.
    Server,
    /// Run Native Shell GUI client (Clippi 1:1 replica).
    #[serde(alias = "gui", alias = "desktop")]
    Gui,
}

/// Server transport and endpoint configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Execution mode (`sync`, `mcp-stdio`, `mcp-http`, or `server`).
    #[serde(default)]
    pub mode: ServerMode,
    /// Bind IP address for the MCP HTTP server (`127.0.0.1` by default).
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port for the MCP HTTP server (`8000` by default).
    #[serde(default = "default_port")]
    pub port: u16,
    /// HTTP path where MCP is mounted (`/mcp` by default).
    #[serde(default = "default_endpoint_path")]
    pub endpoint_path: String,
    /// Whether non-loopback binds are permitted for MCP HTTP (`false` by default).
    #[serde(default)]
    pub allow_remote: bool,
    /// Maximum HTTP request body size in bytes (`16 MiB` by default).
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mode: ServerMode::default(),
            host: default_host(),
            port: default_port(),
            endpoint_path: default_endpoint_path(),
            allow_remote: false,
            max_request_body_bytes: default_max_request_body_bytes(),
        }
    }
}

impl ServerConfig {
    /// Parse `host` and `port` into a [`SocketAddr`].
    ///
    /// # Errors
    ///
    /// Returns an error if `host:port` is not a valid socket address.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host.trim(), self.port)
            .parse::<SocketAddr>()
            .map_err(|e| eyre!("解析 server 地址 `{}:{}` 失败: {e}", self.host, self.port))
    }
}

/// Paths for Skill and Book catalog directories on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Root directory containing Skill subdirectories (`./resource/skills` by default).
    #[serde(default = "default_skills_dir")]
    pub skills: PathBuf,
    /// Root directory containing Book subdirectories (`./resource/books` by default).
    #[serde(default = "default_books_dir")]
    pub books: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            skills: default_skills_dir(),
            books: default_books_dir(),
        }
    }
}

/// `[xerj]` configuration section in `etc/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XerjSectionConfig {
    /// Directory where XERJ stores indices, segments, WAL, and the sync ledger.
    #[serde(default = "default_xerj_data_dir")]
    pub data_dir: PathBuf,
    /// Index name for Skill chunks (`amemory-skills-v1` by default).
    #[serde(default = "default_skills_index")]
    pub skills_index: String,
    /// Index name for Book chunks (`amemory-books-v1` by default).
    #[serde(default = "default_books_index")]
    pub books_index: String,
    /// Index name for Memory chunks (`amemory-memory-v1` by default).
    #[serde(default = "default_memory_index")]
    pub memory_index: String,
    /// Server-side embedding mode (`lexical` by default).
    #[serde(default = "default_embed_mode")]
    pub embed_mode: String,
    /// Bind IP address for XERJ listeners in `server` mode (`127.0.0.1` by default).
    #[serde(default = "default_host")]
    pub bind_address: String,
    /// Elasticsearch-compatible listener port in `server` mode (`9200` by default).
    #[serde(default = "default_es_port")]
    pub es_compat_port: u16,
    /// Native REST listener port in `server` mode (`8080` by default).
    #[serde(default = "default_rest_port")]
    pub rest_port: u16,
    /// gRPC listener port in `server` mode (`8081` by default).
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    /// Optional external XERJ ES-compatible URL override for MCP tool proxying.
    #[serde(default)]
    pub external_es_url: Option<String>,
    /// Optional XERJ Hub API key (`xerj_hub_search`, `xerj_hub_install`, `xerj_hub_publish`).
    /// Falls back to the `XERJ_HUB_API_KEY` environment variable when omitted.
    #[serde(default)]
    pub hub_api_key: Option<String>,
}

impl Default for XerjSectionConfig {
    fn default() -> Self {
        Self {
            data_dir: default_xerj_data_dir(),
            skills_index: default_skills_index(),
            books_index: default_books_index(),
            memory_index: default_memory_index(),
            embed_mode: default_embed_mode(),
            bind_address: default_host(),
            es_compat_port: default_es_port(),
            rest_port: default_rest_port(),
            grpc_port: default_grpc_port(),
            external_es_url: None,
            hub_api_key: None,
        }
    }
}

/// Security limits for document imports into Books.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentImportConfig {
    /// Allowed local filesystem root directories for `file_path` imports (`["./resource/uploads"]` by default).
    #[serde(default = "default_allowed_roots")]
    pub allowed_roots: Vec<PathBuf>,
    /// Maximum allowed document size in bytes (`50 MiB` by default).
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: usize,
}

impl Default for DocumentImportConfig {
    fn default() -> Self {
        Self {
            allowed_roots: default_allowed_roots(),
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

/// `[logger]` section in `etc/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggerSectionConfig {
    /// Log level string (`error`, `warn`, `info`, `debug`, `trace`).
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Maximum log file size in MiB before rotation.
    #[serde(default = "default_log_size_mb")]
    pub size_mb: u64,
    /// Whether to duplicate logs to console when not running in `mcp-stdio` mode.
    #[serde(default = "default_true")]
    pub console: bool,
    /// Directory where log files are written.
    #[serde(default = "default_log_dir")]
    pub dir: String,
}

impl Default for LoggerSectionConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            size_mb: default_log_size_mb(),
            console: true,
            dir: default_log_dir(),
        }
    }
}

/// `[panic]` section in `etc/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanicSectionConfig {
    /// Whether the process should exit after recording a panic.
    #[serde(default = "default_true")]
    pub exit_on_panic: bool,
}

impl Default for PanicSectionConfig {
    fn default() -> Self {
        Self {
            exit_on_panic: true,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

const fn default_log_size_mb() -> u64 {
    10
}

const fn default_true() -> bool {
    true
}

fn default_log_dir() -> String {
    "./logs".to_string()
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

const fn default_port() -> u16 {
    8000
}

fn default_endpoint_path() -> String {
    "/mcp".to_string()
}

const fn default_max_request_body_bytes() -> usize {
    16 * 1024 * 1024
}

fn default_skills_dir() -> PathBuf {
    PathBuf::from("./resource/skills")
}

fn default_books_dir() -> PathBuf {
    PathBuf::from("./resource/books")
}

fn default_allowed_roots() -> Vec<PathBuf> {
    vec![PathBuf::from("./resource/uploads")]
}

fn default_xerj_data_dir() -> PathBuf {
    PathBuf::from("./data/xerj")
}

fn default_skills_index() -> String {
    DEFAULT_SKILLS_INDEX.to_string()
}

fn default_books_index() -> String {
    DEFAULT_BOOKS_INDEX.to_string()
}

fn default_memory_index() -> String {
    DEFAULT_MEMORY_INDEX.to_string()
}

fn default_embed_mode() -> String {
    "lexical".to_string()
}

const fn default_es_port() -> u16 {
    9200
}

const fn default_rest_port() -> u16 {
    8080
}

const fn default_grpc_port() -> u16 {
    8081
}

const fn default_max_file_bytes() -> usize {
    50 * 1024 * 1024
}
