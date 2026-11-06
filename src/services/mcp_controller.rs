//! Background MCP Server lifecycle controller.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use domain::book::BookCatalog;
use domain::skill::SkillCatalog;
use eyre::{Result, WrapErr};
use tokio::sync::watch;
use tracing::{error, info};
use xcat_mcp::{
    AmemoryToolProvider, HttpMcpConfig, UnifiedMcpServer, XerjToolProvider, serve_http,
};

use super::amemory_runtime::AmemoryRuntime;
use crate::config::AppConfig;

/// Controller managing the background MCP HTTP server runtime.
pub struct McpController {
    running: Arc<AtomicBool>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl Default for McpController {
    fn default() -> Self {
        Self::new()
    }
}

impl McpController {
    /// Create a new idle controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
        }
    }

    /// Whether the background MCP server is currently active.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the MCP HTTP server in the background using the runtime's adapter.
    pub fn start(&mut self, runtime: Arc<AmemoryRuntime>) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let config = runtime.config().clone();
        let adapter = runtime.adapter().clone();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            running_flag.store(true, Ordering::SeqCst);
            let loopback = if config.xerj.external_es_url.is_none() {
                match adapter.start_loopback_es_listener().await {
                    Ok(lb) => Some(lb),
                    Err(err) => {
                        error!("Failed to start loopback ES listener: {err:#}");
                        running_flag.store(false, Ordering::SeqCst);
                        return;
                    }
                }
            } else {
                None
            };

            let es_url = config
                .xerj
                .external_es_url
                .as_deref()
                .or_else(|| loopback.as_ref().map(adapter::LoopbackEsServer::base_url))
                .unwrap_or("http://127.0.0.1:9200");

            let skills_catalog = SkillCatalog::new(&config.storage.skills);
            let books_catalog = BookCatalog::new(&config.storage.books);

            let amemory_provider = AmemoryToolProvider::new(
                adapter.clone(),
                skills_catalog,
                books_catalog,
                config.to_import_policy(),
            );
            let xerj_provider = match XerjToolProvider::new(es_url, config.resolve_hub_api_key()) {
                Ok(p) => p,
                Err(err) => {
                    error!("Failed to init XERJ MCP provider: {err:#}");
                    running_flag.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let unified = match UnifiedMcpServer::new(amemory_provider, xerj_provider) {
                Ok(u) => u,
                Err(err) => {
                    error!("Failed to build unified MCP server: {err:#}");
                    running_flag.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let bind_addr = match config.server.socket_addr() {
                Ok(addr) => addr,
                Err(err) => {
                    error!("Failed to resolve bind address: {err:#}");
                    running_flag.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let http_cfg = HttpMcpConfig {
                bind_addr,
                endpoint_path: config.server.endpoint_path.clone(),
                max_request_body_bytes: config.server.max_request_body_bytes,
                allow_remote: config.server.allow_remote,
            };

            info!("Starting background MCP HTTP server on {bind_addr}...");
            tokio::select! {
                res = serve_http(unified, http_cfg) => {
                    if let Err(err) = res {
                        error!("Background MCP HTTP server exited with error: {err:#}");
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("Received shutdown signal for background MCP server");
                }
            }

            if let Some(lb) = loopback {
                lb.shutdown().await;
            }
            running_flag.store(false, Ordering::SeqCst);
        });

        self.shutdown_tx = Some(shutdown_tx);
        Ok(())
    }

    /// Stop the background MCP server.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        self.running.store(false, Ordering::SeqCst);
    }
}
