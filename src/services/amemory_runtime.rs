//! Amemory runtime service managing Skills, Books, Memory, and XERJ search.

use std::path::Path;
use std::sync::Arc;

use adapter::{SearchStore, XerjAdapter};
use domain::book::{BookCatalog, BookEntry, CreateBookInput};
use domain::memory::{
    IndexDocument, IndexMutation, IndexTarget, MemoryChunkInput, SearchHit, SearchRequest,
    StoreStats, SyncReport,
};
use domain::skill::{CreateSkillInput, SkillCatalog, SkillEntry};
use eyre::{Result, WrapErr};

use crate::config::AppConfig;
use crate::core::types::{HealthLevel, ItemKind, KnowledgeItem, StatsSnapshot};

/// Thread-safe runtime wrapper for Amemory domain operations and search engine.
#[derive(Clone)]
pub struct AmemoryRuntime {
    config: Arc<AppConfig>,
    adapter: Arc<XerjAdapter>,
    skills_catalog: Arc<SkillCatalog>,
    books_catalog: Arc<BookCatalog>,
}

impl AmemoryRuntime {
    /// Initialize a new runtime instance from application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if opening XERJ adapter fails.
    pub fn new(config: AppConfig) -> Result<Self> {
        let skills_catalog = Arc::new(SkillCatalog::new(&config.storage.skills));
        let books_catalog = Arc::new(BookCatalog::new(&config.storage.books));
        let adapter = Arc::new(
            XerjAdapter::open(config.to_xerj_config())
                .wrap_err("初始化 XERJ 引擎适配器失败")?,
        );
        Ok(Self {
            config: Arc::new(config),
            adapter,
            skills_catalog,
            books_catalog,
        })
    }

    /// Access the current application configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Access the underlying XERJ adapter.
    #[must_use]
    pub fn adapter(&self) -> &Arc<XerjAdapter> {
        &self.adapter
    }

    /// List all Skills on disk converted to [`KnowledgeItem`].
    pub fn list_skills(&self) -> Result<Vec<KnowledgeItem>> {
        let summaries = self.skills_catalog.list_skills()?;
        let mut items = Vec::with_capacity(summaries.len());
        for s in summaries {
            items.push(KnowledgeItem {
                id: format!("skill:{}", s.name),
                kind: ItemKind::Skill,
                title: s.name.clone(),
                uri: format!("skill://{}/SKILL.md", s.name),
                preview: s.description,
                tags: vec![format!("skill:{}", s.name)],
                owner: s.name,
                relative_path: String::new(),
                part_index: 1,
                part_total: 1,
                file_count: s.file_count,
                score: None,
                updated_at: None,
                favorite: false,
            });
        }
        Ok(items)
    }

    /// List all Books on disk converted to [`KnowledgeItem`].
    pub fn list_books(&self) -> Result<Vec<KnowledgeItem>> {
        let summaries = self.books_catalog.list_books()?;
        let mut items = Vec::with_capacity(summaries.len());
        for b in summaries {
            items.push(KnowledgeItem {
                id: format!("book:{}", b.name),
                kind: ItemKind::Book,
                title: b.name.clone(),
                uri: format!("book://{}/BOOK.md", b.name),
                preview: b.description,
                tags: vec![format!("book:{}", b.name)],
                owner: b.name,
                relative_path: String::new(),
                part_index: 1,
                part_total: 1,
                file_count: b.file_count,
                score: None,
                updated_at: None,
                favorite: false,
            });
        }
        Ok(items)
    }

    /// Execute search query across Amemory indices.
    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<KnowledgeItem>> {
        let hits = self.adapter.search(request).await?;
        let mut items = Vec::with_capacity(hits.len());
        for hit in hits {
            let (owner, relative_path, kind) = parse_uri_and_tags(&hit);
            items.push(KnowledgeItem {
                id: hit.doc_id,
                kind,
                title: hit.title.unwrap_or_else(|| hit.uri.clone().unwrap_or_default()),
                uri: hit.uri.unwrap_or_default(),
                preview: hit.text,
                tags: hit.tags,
                owner,
                relative_path,
                part_index: 1,
                part_total: 1,
                file_count: 0,
                score: hit.score,
                updated_at: None,
                favorite: false,
            });
        }
        Ok(items)
    }

    /// Read live system statistics snapshot.
    pub async fn stats_snapshot(&self) -> StatsSnapshot {
        let skills_list = self.skills_catalog.list_skills().unwrap_or_default();
        let skills_count = skills_list.len();
        let skill_files_count = skills_list.iter().map(|s| s.file_count).sum();

        let books_list = self.books_catalog.list_books().unwrap_or_default();
        let books_count = books_list.len();
        let book_files_count = books_list.iter().map(|b| b.file_count).sum();

        let store_stats = self.adapter.stats().await.unwrap_or(StoreStats {
            skills_docs: 0,
            books_docs: 0,
            memory_docs: 0,
            data_dir: self.config.xerj.data_dir.display().to_string(),
        });

        let data_dir_path = &self.config.xerj.data_dir;
        let disk_usage_bytes = calculate_dir_size(data_dir_path);

        StatsSnapshot {
            skills_count,
            skill_files_count,
            skills_docs: store_stats.skills_docs,
            books_count,
            book_files_count,
            imported_docs_count: 0,
            books_docs: store_stats.books_docs,
            memory_docs: store_stats.memory_docs,
            namespace_count: 1,
            health: HealthLevel::Green,
            index_count: 3,
            total_docs: store_stats.skills_docs + store_stats.books_docs + store_stats.memory_docs,
            disk_usage_bytes,
            embed_mode: self.config.xerj.embed_mode.clone(),
            last_search_latency_ms: None,
            last_sync_summary: None,
            data_dir: store_stats.data_dir,
        }
    }

    /// Synchronize all disk catalogs (Skills and Books) into XERJ.
    pub async fn sync_all(&self) -> Result<(SyncReport, SyncReport)> {
        let (scanned_skills, skill_docs) = self.skills_catalog.build_all_index_documents()?;
        let skill_sync = self
            .adapter
            .sync_catalog(IndexTarget::Skills, scanned_skills, &skill_docs)
            .await?;

        let (scanned_books, book_docs) = self.books_catalog.build_all_index_documents()?;
        let book_sync = self
            .adapter
            .sync_catalog(IndexTarget::Books, scanned_books, &book_docs)
            .await?;

        Ok((skill_sync, book_sync))
    }

    /// Create or update a Skill.
    pub async fn save_skill(&self, input: &CreateSkillInput) -> Result<()> {
        let created = self.skills_catalog.create_skill(input)?;
        let docs = self.skills_catalog.build_skill_documents(&created.skill)?;
        let mutation = IndexMutation {
            target: IndexTarget::Skills,
            owner_prefix: Some(format!("skill:{}:", created.skill.name)),
            upserts: docs,
            delete_doc_ids: Vec::new(),
        };
        self.adapter.apply_mutation(&mutation).await?;
        Ok(())
    }

    /// Create or update a Book.
    pub async fn save_book(&self, input: &CreateBookInput) -> Result<()> {
        let created = self.books_catalog.create_book(input)?;
        let docs = self.books_catalog.build_book_documents(&created.book)?;
        let mutation = IndexMutation {
            target: IndexTarget::Books,
            owner_prefix: Some(format!("book:{}:", created.book.name)),
            upserts: docs,
            delete_doc_ids: Vec::new(),
        };
        self.adapter.apply_mutation(&mutation).await?;
        Ok(())
    }

    /// Append a memory chunk directly into XERJ.
    pub async fn add_memory_chunk(&self, chunk: MemoryChunkInput) -> Result<String> {
        let ids = self.adapter.add_memory_chunks(&[chunk]).await?;
        ids.into_iter().next().ok_or_else(|| eyre::eyre!("写入记忆分片未返回 ID"))
    }

    /// Delete a Skill or Book from disk and remove its documents from the index.
    pub async fn delete_item(&self, kind: ItemKind, name: &str) -> Result<()> {
        match kind {
            ItemKind::Skill => {
                self.skills_catalog.delete_skill(name)?;
                let mutation = IndexMutation {
                    target: IndexTarget::Skills,
                    owner_prefix: Some(format!("skill:{name}:")),
                    upserts: Vec::new(),
                    delete_doc_ids: Vec::new(),
                };
                self.adapter.apply_mutation(&mutation).await?;
            }
            ItemKind::Book => {
                self.books_catalog.delete_book(name)?;
                let mutation = IndexMutation {
                    target: IndexTarget::Books,
                    owner_prefix: Some(format!("book:{name}:")),
                    upserts: Vec::new(),
                    delete_doc_ids: Vec::new(),
                };
                self.adapter.apply_mutation(&mutation).await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Force flush XERJ in-memory data to disk.
    pub async fn flush_all(&self) {
        self.adapter.flush_all().await;
    }
}

fn parse_uri_and_tags(hit: &SearchHit) -> (String, String, ItemKind) {
    let uri = hit.uri.as_deref().unwrap_or_default();
    let mut owner = String::new();
    let mut relative_path = String::new();

    if let Some(rest) = uri.strip_prefix("skill://") {
        if let Some((o, p)) = rest.split_once('/') {
            owner = o.to_string();
            relative_path = p.to_string();
        } else {
            owner = rest.to_string();
        }
    } else if let Some(rest) = uri.strip_prefix("book://") {
        if let Some((o, p)) = rest.split_once('/') {
            owner = o.to_string();
            relative_path = p.to_string();
        } else {
            owner = rest.to_string();
        }
    } else if let Some(rest) = uri.strip_prefix("memory://") {
        owner = rest.split('/').next().unwrap_or("default").to_string();
    }

    let kind = ItemKind::classify(&hit.index, &relative_path, uri);
    (owner, relative_path, kind)
}

fn calculate_dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    size += calculate_dir_size(&entry.path());
                } else {
                    size += meta.len();
                }
            }
        }
    }
    size
}
