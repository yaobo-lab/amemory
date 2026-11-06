//! XERJ Engine and API isolation adapter for Amemory.
//!
//! This crate encapsulates all interactions with `xerj-engine`, `xerj-api`,
//! `xerj-common`, `xerj-query`, and `xerj-server` behind the [`SearchStore`]
//! trait and [`XerjAdapter`] so that neither `amemory-domain` nor `amemory-mcp`
//! depends on XERJ internal types.

pub mod engine;
pub mod index;
pub mod mapping;
pub mod search;

use std::future::Future;

use domain::DomainError;
use domain::memory::{
    IndexDocument, IndexMutation, IndexTarget, MemoryChunkInput, SearchHit, SearchRequest,
    StoreStats, SyncReport,
};
use thiserror::Error;

pub use engine::{
    DEFAULT_BOOKS_INDEX, DEFAULT_MEMORY_INDEX, DEFAULT_SKILLS_INDEX, LoopbackEsServer,
    RunningXerjServer, XerjAdapter, XerjConfig,
};

/// Structured error returned by the XERJ adapter layer.
#[derive(Debug, Error)]
pub enum AdapterError {
    /// Domain validation or filesystem error.
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Index schema or field mapping error.
    #[error("XERJ 索引 Schema 错误: {0}")]
    Schema(String),

    /// Query DSL parsing or planning error.
    #[error("XERJ 查询解析错误: {0}")]
    Query(String),

    /// Underlying XERJ engine or storage error.
    #[error("XERJ 引擎执行错误: {0}")]
    Engine(String),

    /// Serialization or ledger format error.
    #[error("序列化错误: {0}")]
    Serialization(String),

    /// Filesystem or network I/O error.
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// Narrow storage and search abstraction consumed by upper layers (`amemory-mcp` and `amemory-app`).
pub trait SearchStore: Send + Sync {
    /// Synchronize a full catalog snapshot (`Skills` or `Books`) idempotently.
    fn sync_catalog(
        &self,
        target: IndexTarget,
        scanned_items: usize,
        documents: &[IndexDocument],
    ) -> impl Future<Output = Result<SyncReport, AdapterError>> + Send;

    /// Apply an incremental [`IndexMutation`] (upserts and/or deletions).
    fn apply_mutation(
        &self,
        mutation: &IndexMutation,
    ) -> impl Future<Output = Result<usize, AdapterError>> + Send;

    /// Write a batch of general memory chunks into the Memory index and return their document IDs.
    fn add_memory_chunks(
        &self,
        chunks: &[MemoryChunkInput],
    ) -> impl Future<Output = Result<Vec<String>, AdapterError>> + Send;

    /// Execute a lexical, semantic, or hybrid search across Amemory indices.
    fn search(
        &self,
        request: &SearchRequest,
    ) -> impl Future<Output = Result<Vec<SearchHit>, AdapterError>> + Send;

    /// Return live document counts across the Skills, Books, and Memory indices.
    fn stats(&self) -> impl Future<Output = Result<StoreStats, AdapterError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::memory::{SearchMode, split_text_into_parts};
    use domain::skill::{CreateSkillInput, SkillCatalog, SkillFileInput};
    use tempfile::TempDir;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_sync_mutation_search_and_restart_persistence_work() {
        let dir = TempDir::new().expect("tempdir");
        let skills_dir = dir.path().join("skills");
        let xerj_dir = dir.path().join("xerj");

        let catalog = SkillCatalog::new(&skills_dir);
        let created = catalog
            .create_skill(&CreateSkillInput {
                name: Some("rust-async".to_string()),
                description: Some("Async Rust patterns and Tokio best practices".to_string()),
                body: Some(
                    "# Rust Async\n\nAvoid holding std::sync::MutexGuard across await points."
                        .to_string(),
                ),
                skill_markdown: None,
                files: vec![SkillFileInput {
                    relative_path: "references/tokio.md".to_string(),
                    content: "Use spawn_blocking for CPU-intensive work.".to_string(),
                }],
            })
            .expect("create skill");

        let config = XerjConfig {
            data_dir: xerj_dir.clone(),
            ..XerjConfig::default()
        };

        {
            let adapter = XerjAdapter::open(config.clone()).expect("open adapter");
            let (scanned, docs) = catalog.build_all_index_documents().expect("build docs");
            let sync1 = adapter
                .sync_catalog(IndexTarget::Skills, scanned, &docs)
                .await
                .expect("first sync");
            assert_eq!(sync1.scanned_items, 1);
            assert_eq!(sync1.indexed_documents, 2);
            assert_eq!(sync1.unchanged_documents, 0);

            // Second sync with identical content must skip all documents.
            let sync2 = adapter
                .sync_catalog(IndexTarget::Skills, scanned, &docs)
                .await
                .expect("second sync");
            assert_eq!(sync2.indexed_documents, 0);
            assert_eq!(sync2.unchanged_documents, 2);

            // Write a memory chunk and search across indices.
            let chunk_ids = adapter
                .add_memory_chunks(&[MemoryChunkInput {
                    id: Some("m1".to_string()),
                    namespace: "default".to_string(),
                    title: Some("Mutex note".to_string()),
                    uri: None,
                    text: "Never hold a synchronous MutexGuard across an await boundary."
                        .to_string(),
                    tags: vec!["topic:async".to_string()],
                    metadata: None,
                }])
                .await
                .expect("add memory chunk");
            assert_eq!(chunk_ids, vec!["memory:default:m1".to_string()]);

            for mode in [
                SearchMode::Lexical,
                SearchMode::Semantic,
                SearchMode::Hybrid,
            ] {
                let hits = adapter
                    .search(&SearchRequest {
                        query: "MutexGuard await".to_string(),
                        limit: 10,
                        snippet_chars: 200,
                        mode,
                        uri: None,
                        scope: None,
                        exclude_uris: Vec::new(),
                        exclude_scopes: Vec::new(),
                        tag_filters: Vec::new(),
                    })
                    .await
                    .expect("search");
                assert!(!hits.is_empty(), "expected hits for mode {mode:?}");
            }

            let stats = adapter.stats().await.expect("stats");
            assert_eq!(stats.skills_docs, 2);
            assert_eq!(stats.memory_docs, 1);
            assert_eq!(created.synced_skill_parts, 2);
            assert!(!split_text_into_parts("hello", 100).is_empty());
            adapter.flush_all().await;
        }

        // Reopen adapter on the same data_dir to verify persistence across restarts.
        let reopened = XerjAdapter::open(config).expect("reopen adapter");
        let stats = reopened.stats().await.expect("stats after reopen");
        assert_eq!(stats.skills_docs, 2);
        assert_eq!(stats.memory_docs, 1);
    }
}
