//! XERJ Engine lifecycle, loopback ES-compat listener, and full server bootstrap wrapper.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use amemory_domain::memory::{
    IndexDocument, IndexMutation, IndexTarget, MemoryChunkInput, SearchHit, SearchRequest,
    StoreStats, SyncReport, build_memory_documents,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::error;
use xerj_api::{AppState, build_es_compat_router_with_hub};
use xerj_common::config::Config;
use xerj_common::metrics::Metrics;
use xerj_engine::Engine;
use xerj_server::EmbeddedServer;

use crate::index::{LedgerStore, apply_index_mutation, ensure_indices, sync_catalog_documents};
use crate::search::execute_search;
use crate::{AdapterError, SearchStore};

/// Default index name for Skill chunks.
pub const DEFAULT_SKILLS_INDEX: &str = "amemory-skills-v1";
/// Default index name for Book and imported Book document chunks.
pub const DEFAULT_BOOKS_INDEX: &str = "amemory-books-v1";
/// Default index name for general Memory chunks.
pub const DEFAULT_MEMORY_INDEX: &str = "amemory-memory-v1";

/// Configuration for [`XerjAdapter`] and its embedded XERJ runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XerjConfig {
    /// Directory where XERJ stores segments, WAL, and the Amemory sync ledger.
    pub data_dir: PathBuf,
    /// Index name for Skill documents.
    pub skills_index: String,
    /// Index name for Book documents.
    pub books_index: String,
    /// Index name for Memory chunks.
    pub memory_index: String,
    /// Server-side embedding mode (`lexical` by default).
    pub embed_mode: String,
    /// Bind IP address for XERJ listeners in `server` mode (`127.0.0.1` by default).
    pub bind_address: String,
    /// Elasticsearch-compatible HTTP port in `server` mode (`9200` by default).
    pub es_compat_port: u16,
    /// Native REST HTTP port in `server` mode (`8080` by default).
    pub rest_port: u16,
    /// gRPC port in `server` mode (`8081` by default).
    pub grpc_port: u16,
    /// Optional external XERJ base URL override for MCP tool proxying.
    pub external_es_url: Option<String>,
}

impl Default for XerjConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/xerj"),
            skills_index: DEFAULT_SKILLS_INDEX.to_string(),
            books_index: DEFAULT_BOOKS_INDEX.to_string(),
            memory_index: DEFAULT_MEMORY_INDEX.to_string(),
            embed_mode: "lexical".to_string(),
            bind_address: "127.0.0.1".to_string(),
            es_compat_port: 9200,
            rest_port: 8080,
            grpc_port: 8081,
            external_es_url: None,
        }
    }
}

/// Handle to an ephemeral loopback ES-compatible HTTP listener sharing the adapter's engine.
#[derive(Debug)]
pub struct LoopbackEsServer {
    base_url: String,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl LoopbackEsServer {
    /// Return the `http://127.0.0.1:<port>` base URL of this loopback listener.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Gracefully stop the loopback listener.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.join_handle.await;
    }
}

/// Handle to a full embedded XERJ server runtime (REST + ES-compat + Console + gRPC).
pub struct RunningXerjServer {
    inner: EmbeddedServer,
}

impl RunningXerjServer {
    /// Return the bound address of the Native REST listener.
    #[must_use]
    pub fn rest_addr(&self) -> SocketAddr {
        self.inner.addrs().rest
    }

    /// Return the bound address of the ES-compatible listener.
    #[must_use]
    pub fn es_compat_addr(&self) -> SocketAddr {
        self.inner.addrs().es_compat
    }

    /// Return the bound address of the gRPC listener.
    #[must_use]
    pub fn grpc_addr(&self) -> SocketAddr {
        self.inner.addrs().grpc
    }

    /// Return the base `http://host:port` URL of the ES-compatible listener.
    #[must_use]
    pub fn es_base_url(&self) -> String {
        self.inner.es_base_url()
    }

    /// Wait until the XERJ server shuts down.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if the server task fails.
    pub async fn wait(self) -> Result<(), AdapterError> {
        self.inner
            .wait()
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))
    }

    /// Trigger graceful shutdown and wait for completion.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if shutdown fails.
    pub async fn shutdown(self) -> Result<(), AdapterError> {
        self.inner
            .shutdown()
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))
    }
}

/// Concrete XERJ-backed implementation of [`SearchStore`].
#[derive(Clone)]
pub struct XerjAdapter {
    config: XerjConfig,
    engine: Arc<Engine>,
    ledger: Arc<LedgerStore>,
}

impl std::fmt::Debug for XerjAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XerjAdapter")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl XerjAdapter {
    /// Open or initialize the XERJ engine and Amemory indices in `config.data_dir`.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if opening the engine, ledger, or indices fails.
    pub fn open(config: XerjConfig) -> Result<Self, AdapterError> {
        std::fs::create_dir_all(&config.data_dir)?;
        let ledger = Arc::new(LedgerStore::open(&config.data_dir)?);
        let engine_cfg = Self::build_engine_config(&config);
        let engine =
            Arc::new(Engine::new(engine_cfg).map_err(|e| AdapterError::Engine(e.to_string()))?);
        ensure_indices(
            &engine,
            &config.skills_index,
            &config.books_index,
            &config.memory_index,
        )?;

        Ok(Self {
            config,
            engine,
            ledger,
        })
    }

    /// Return the adapter's [`XerjConfig`].
    #[must_use]
    pub const fn config(&self) -> &XerjConfig {
        &self.config
    }

    /// Return the concrete index name corresponding to `target`.
    #[must_use]
    pub fn index_name_for(&self, target: IndexTarget) -> &str {
        match target {
            IndexTarget::Skills => &self.config.skills_index,
            IndexTarget::Books => &self.config.books_index,
            IndexTarget::Memory => &self.config.memory_index,
        }
    }

    /// Start an ephemeral loopback ES-compatible HTTP listener backed by this adapter's engine.
    ///
    /// This allows `xerj-mcp` tools to proxy requests to the in-process XERJ engine
    /// in `mcp-stdio` and `mcp-http` modes without requiring an external XERJ daemon.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if binding the loopback socket or initializing metrics fails.
    pub async fn start_loopback_es_listener(&self) -> Result<LoopbackEsServer, AdapterError> {
        let mut cfg = Self::build_engine_config(&self.config);
        cfg.auth.enabled = false;

        let metrics = Metrics::new().map_err(|e| AdapterError::Engine(e.to_string()))?;
        let state = AppState::new(cfg, (*self.engine).clone(), metrics);
        let hub_shared = xerj_api::hub_api::HubShared::new(state.clone());
        let router = build_es_compat_router_with_hub(state, hub_shared);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let bound_addr = listener.local_addr()?;
        let base_url = format!("http://{bound_addr}");

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = tokio::spawn(async move {
            let svc = router.into_make_service_with_connect_info::<SocketAddr>();
            if let Err(err) = axum::serve(listener, svc)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.wait_for(|&v| v).await;
                })
                .await
            {
                error!("loopback XERJ ES listener error: {err:#}");
            }
        });

        Ok(LoopbackEsServer {
            base_url,
            shutdown_tx,
            join_handle,
        })
    }

    /// Start the full XERJ server runtime (Native REST + ES-compat + Console + gRPC)
    /// sharing this adapter's in-process engine.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if binding listeners or bootstrapping the server fails.
    pub async fn start_full_server(&self) -> Result<RunningXerjServer, AdapterError> {
        let mut cfg = Self::build_engine_config(&self.config);
        cfg.auth.enabled = false;
        let inner = xerj_server::start_with_engine(cfg, self.engine.clone())
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        Ok(RunningXerjServer { inner })
    }

    /// Force a synchronous flush of all in-memory index state to disk.
    pub async fn flush_all(&self) {
        self.engine.flush_all_force().await;
    }

    fn build_engine_config(config: &XerjConfig) -> Config {
        let mut cfg = Config::default();
        cfg.server.data_dir = config.data_dir.to_string_lossy().into_owned();
        cfg.server.bind_address.clone_from(&config.bind_address);
        cfg.server.es_compat_port = config.es_compat_port;
        cfg.server.rest_port = config.rest_port;
        cfg.server.grpc_port = config.grpc_port;
        cfg.auth.enabled = false;
        cfg.embedding.mode.clone_from(&config.embed_mode);
        cfg
    }
}

impl SearchStore for XerjAdapter {
    async fn sync_catalog(
        &self,
        target: IndexTarget,
        scanned_items: usize,
        documents: &[IndexDocument],
    ) -> Result<SyncReport, AdapterError> {
        let index_name = self.index_name_for(target);
        sync_catalog_documents(
            &self.engine,
            &self.ledger,
            target,
            index_name,
            scanned_items,
            documents,
        )
        .await
    }

    async fn apply_mutation(&self, mutation: &IndexMutation) -> Result<usize, AdapterError> {
        let index_name = self.index_name_for(mutation.target);
        apply_index_mutation(&self.engine, &self.ledger, index_name, mutation).await
    }

    async fn add_memory_chunks(
        &self,
        chunks: &[MemoryChunkInput],
    ) -> Result<Vec<String>, AdapterError> {
        let documents = build_memory_documents(chunks)?;
        let ids: Vec<String> = documents.iter().map(|d| d.doc_id.clone()).collect();
        let mutation = IndexMutation {
            target: IndexTarget::Memory,
            owner_prefix: None,
            upserts: documents,
            delete_doc_ids: Vec::new(),
        };
        self.apply_mutation(&mutation).await?;
        Ok(ids)
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchHit>, AdapterError> {
        let validated = request.clone().validated()?;
        execute_search(
            &self.engine,
            &self.config.skills_index,
            &self.config.books_index,
            &self.config.memory_index,
            &validated,
        )
        .await
    }

    async fn stats(&self) -> Result<StoreStats, AdapterError> {
        let skills_docs = self
            .engine
            .index_stats(&self.config.skills_index)
            .await
            .map_or(0, |s| s.doc_count);
        let books_docs = self
            .engine
            .index_stats(&self.config.books_index)
            .await
            .map_or(0, |s| s.doc_count);
        let memory_docs = self
            .engine
            .index_stats(&self.config.memory_index)
            .await
            .map_or(0, |s| s.doc_count);

        Ok(StoreStats {
            skills_docs,
            books_docs,
            memory_docs,
            data_dir: self.config.data_dir.display().to_string(),
        })
    }
}
