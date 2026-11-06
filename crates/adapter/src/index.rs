//! Index lifecycle, idempotent catalog synchronization, and mutation application over XERJ.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use domain::memory::{
    IndexDocument, IndexMutation, IndexTarget, SyncReport, book_document_doc_prefix,
};
use serde::{Deserialize, Serialize};
use xerj_engine::Engine;

use crate::AdapterError;
use crate::mapping::{build_index_schema, document_to_source};

const LEDGER_FILE_NAME: &str = "amemory-ledger.json";

/// Persistent content-hash ledger for idempotent synchronization.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncLedger {
    /// Map from concrete index name to (`doc_id -> content_hash`).
    #[serde(default)]
    pub indices: BTreeMap<String, BTreeMap<String, String>>,
}

/// Manages ledger state on disk under `<data_dir>/amemory-ledger.json`.
#[derive(Debug)]
pub struct LedgerStore {
    path: PathBuf,
    inner: Mutex<SyncLedger>,
}

impl LedgerStore {
    /// Load an existing ledger from `data_dir` or initialize an empty one.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if the directory cannot be created or the ledger file is corrupted.
    pub fn open(data_dir: &Path) -> Result<Self, AdapterError> {
        fs::create_dir_all(data_dir)?;
        let path = data_dir.join(LEDGER_FILE_NAME);
        let ledger = if path.is_file() {
            let content = fs::read_to_string(&path)?;
            if content.trim().is_empty() {
                SyncLedger::default()
            } else {
                serde_json::from_str(&content)
                    .map_err(|e| AdapterError::Serialization(e.to_string()))?
            }
        } else {
            SyncLedger::default()
        };

        Ok(Self {
            path,
            inner: Mutex::new(ledger),
        })
    }

    /// Snapshot the recorded `(doc_id -> content_hash)` map for `index_name`.
    pub fn snapshot_index(&self, index_name: &str) -> BTreeMap<String, String> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.indices.get(index_name).cloned().unwrap_or_default()
    }

    /// Replace the recorded `(doc_id -> content_hash)` map for `index_name` and persist to disk.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if serializing or writing the ledger fails.
    pub fn save_index(
        &self,
        index_name: &str,
        entries: BTreeMap<String, String>,
    ) -> Result<(), AdapterError> {
        let serialized = {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.indices.insert(index_name.to_string(), entries);
            serde_json::to_string_pretty(&*guard)
                .map_err(|e| AdapterError::Serialization(e.to_string()))?
        };
        fs::write(&self.path, serialized)?;
        Ok(())
    }
}

/// Ensure all three Amemory indices exist in `engine` with their configured schemas.
///
/// # Errors
///
/// Returns [`AdapterError`] if index creation fails.
pub fn ensure_indices(
    engine: &Engine,
    skills_index: &str,
    books_index: &str,
    memory_index: &str,
) -> Result<(), AdapterError> {
    for (target, name) in [
        (IndexTarget::Skills, skills_index),
        (IndexTarget::Books, books_index),
        (IndexTarget::Memory, memory_index),
    ] {
        if engine.get_index(name).is_err() {
            let schema = build_index_schema(target)?;
            engine
                .create_index(name, schema)
                .map_err(|e| AdapterError::Engine(e.to_string()))?;
        }
    }
    Ok(())
}

/// Synchronize a complete catalog snapshot into `index_name` idempotently.
///
/// # Errors
///
/// Returns [`AdapterError`] if indexing, deletion, refresh, flush, or ledger persistence fails.
pub async fn sync_catalog_documents(
    engine: &Engine,
    ledger: &LedgerStore,
    target: IndexTarget,
    index_name: &str,
    scanned_items: usize,
    documents: &[IndexDocument],
) -> Result<SyncReport, AdapterError> {
    let idx = engine
        .get_index(index_name)
        .map_err(|e| AdapterError::Engine(e.to_string()))?;

    let mut recorded = ledger.snapshot_index(index_name);
    let stats = idx.stats().await;
    if stats.doc_count == 0 && !recorded.is_empty() {
        recorded.clear();
    }

    let mut incoming_ids = BTreeSet::new();
    let mut indexed_documents = 0usize;
    let mut unchanged_documents = 0usize;

    for doc in documents {
        incoming_ids.insert(doc.doc_id.clone());
        let is_unchanged = recorded
            .get(&doc.doc_id)
            .is_some_and(|prev_hash| prev_hash == &doc.content_hash);

        if is_unchanged {
            unchanged_documents += 1;
            continue;
        }

        let source = document_to_source(doc);
        idx.index_document(Some(doc.doc_id.clone()), source)
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        recorded.insert(doc.doc_id.clone(), doc.content_hash.clone());
        indexed_documents += 1;
    }

    let stale_ids: Vec<String> = recorded
        .keys()
        .filter(|id| !incoming_ids.contains(*id))
        .cloned()
        .collect();

    let mut deleted_documents = 0usize;
    for stale_id in stale_ids {
        let _ = idx
            .delete_document(&stale_id)
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        recorded.remove(&stale_id);
        deleted_documents += 1;
    }

    if indexed_documents > 0 || deleted_documents > 0 {
        idx.refresh()
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        idx.flush()
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        ledger.save_index(index_name, recorded)?;
    }

    let message = format!(
        "{} 同步完成: 扫描 {} 项，写入/更新 {} 个分片，未变更 {} 个分片，清理过期 {} 个分片",
        target.as_str(),
        scanned_items,
        indexed_documents,
        unchanged_documents,
        deleted_documents
    );

    Ok(SyncReport {
        scanned_items,
        indexed_documents,
        unchanged_documents,
        deleted_documents,
        message,
    })
}

/// Apply an incremental [`IndexMutation`] (for a single Skill, Book, or Memory batch) to `index_name`.
///
/// # Errors
///
/// Returns [`AdapterError`] if indexing, deletion, refresh, flush, or ledger persistence fails.
pub async fn apply_index_mutation(
    engine: &Engine,
    ledger: &LedgerStore,
    index_name: &str,
    mutation: &IndexMutation,
) -> Result<usize, AdapterError> {
    let idx = engine
        .get_index(index_name)
        .map_err(|e| AdapterError::Engine(e.to_string()))?;

    let mut recorded = ledger.snapshot_index(index_name);
    let upsert_ids: BTreeSet<&str> = mutation.upserts.iter().map(|d| d.doc_id.as_str()).collect();

    let mut to_delete: BTreeSet<String> = mutation.delete_doc_ids.iter().cloned().collect();

    if let Some(prefix) = &mutation.owner_prefix {
        for existing_id in recorded.keys() {
            if existing_id.starts_with(prefix) && !upsert_ids.contains(existing_id.as_str()) {
                to_delete.insert(existing_id.clone());
            }
        }
        if mutation.target == IndexTarget::Books
            && let Some(book_name) = prefix
                .strip_prefix("book:")
                .and_then(|s| s.strip_suffix(':'))
        {
            let doc_prefix = book_document_doc_prefix(book_name);
            for existing_id in recorded.keys() {
                if existing_id.starts_with(&doc_prefix)
                    && !upsert_ids.contains(existing_id.as_str())
                {
                    to_delete.insert(existing_id.clone());
                }
            }
        }
    }

    for doc_id in &to_delete {
        let _ = idx
            .delete_document(doc_id)
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        recorded.remove(doc_id);
    }

    let mut written = 0usize;
    for doc in &mutation.upserts {
        let source = document_to_source(doc);
        idx.index_document(Some(doc.doc_id.clone()), source)
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;
        recorded.insert(doc.doc_id.clone(), doc.content_hash.clone());
        written += 1;
    }

    idx.refresh()
        .await
        .map_err(|e| AdapterError::Engine(e.to_string()))?;
    idx.flush()
        .await
        .map_err(|e| AdapterError::Engine(e.to_string()))?;
    ledger.save_index(index_name, recorded)?;

    Ok(written)
}
