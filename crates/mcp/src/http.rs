//! Streamable HTTP transport for the unified MCP server.

use std::net::SocketAddr;
use std::sync::Arc;

use adapter::SearchStore;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;

use crate::McpFacadeError;
use crate::registry::UnifiedMcpServer;

/// Configuration for the Streamable HTTP MCP transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpMcpConfig {
    /// Socket address to bind (`127.0.0.1:8000` by default).
    pub bind_addr: SocketAddr,
    /// Route path where the MCP service is mounted (`/mcp` by default).
    pub endpoint_path: String,
    /// Maximum allowed HTTP request body size in bytes.
    pub max_request_body_bytes: usize,
    /// Whether non-loopback bind addresses are permitted.
    pub allow_remote: bool,
}

impl Default for HttpMcpConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 8000)),
            endpoint_path: "/mcp".to_string(),
            max_request_body_bytes: 16 * 1024 * 1024,
            allow_remote: false,
        }
    }
}

/// Build an [`axum::Router`] serving the [`UnifiedMcpServer`] at `config.endpoint_path`.
///
/// # Errors
///
/// Returns [`McpFacadeError::Transport`] if `config.endpoint_path` is invalid or
/// a non-loopback address is configured while `allow_remote` is `false`.
pub fn build_http_router<S: SearchStore + Clone + 'static>(
    server: UnifiedMcpServer<S>,
    config: &HttpMcpConfig,
) -> Result<Router, McpFacadeError> {
    if !config.allow_remote && !config.bind_addr.ip().is_loopback() {
        return Err(McpFacadeError::Transport(format!(
            "拒绝在非回环地址 `{}` 上启动未授权的 HTTP MCP 服务（如需开启，请显式设置 server.allow_remote = true）",
            config.bind_addr
        )));
    }

    let endpoint = config.endpoint_path.trim();
    if !endpoint.starts_with('/') {
        return Err(McpFacadeError::Transport(format!(
            "HTTP endpoint_path `{endpoint}` 必须以 `/` 开头"
        )));
    }

    let service: StreamableHttpService<UnifiedMcpServer<S>, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );

    let router = Router::new()
        .nest_service(endpoint, service)
        .layer(DefaultBodyLimit::max(config.max_request_body_bytes))
        .layer(RequestBodyLimitLayer::new(config.max_request_body_bytes));

    Ok(router)
}

/// Bind and serve the unified HTTP MCP router until Ctrl-C is received.
///
/// # Errors
///
/// Returns [`McpFacadeError::Transport`] if binding or serving fails.
pub async fn serve_http<S: SearchStore + Clone + 'static>(
    server: UnifiedMcpServer<S>,
    config: HttpMcpConfig,
) -> Result<(), McpFacadeError> {
    let router = build_http_router(server, &config)?;
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .map_err(|e| McpFacadeError::Transport(format!("绑定 {} 失败: {e}", config.bind_addr)))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| McpFacadeError::Transport(format!("获取本地监听地址失败: {e}")))?;
    tracing::info!(
        "HTTP MCP listener bound: http://{}{}",
        local_addr,
        config.endpoint_path
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| McpFacadeError::Transport(e.to_string()))
}
