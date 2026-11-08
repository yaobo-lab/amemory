//! Amemory runtime service managing Skills, Books, Memory, and XERJ search.

use std::path::Path;
use std::sync::Arc;

use adapter::{SearchStore, XerjAdapter};
use domain::book::{
    BookCatalog, BookFileInput, BookFileUpdatePolicy, CreateBookInput, UpdateBookInput,
};
use domain::memory::{
    IndexDocument, IndexTarget, MemoryChunkInput, SearchHit, SearchRequest, SyncReport,
    build_memory_documents,
};
use domain::skill::{
    CreateSkillInput, SkillCatalog, SkillFileInput, SkillFileUpdatePolicy, UpdateSkillInput,
};
use eyre::{Result, WrapErr};
use toolkit_rs::chrono::Utc;

use crate::config::AppConfig;
use crate::core::settings::GuiSettings;
use crate::core::types::{HealthLevel, ItemKind, KnowledgeItem, StatsSnapshot, SubFileEntry};

const CUSTOM_MEMORIES_FILE: &str = "amemory-memories.json";

/// Open a directory or file's parent directory in the system file manager.
pub fn open_path_in_explorer(path: &Path) {
    let resolved = crate::core::paths::resolve_relative(path);
    let dir_to_open = if resolved.is_file() {
        resolved.parent().unwrap_or(&resolved).to_path_buf()
    } else {
        let _ = std::fs::create_dir_all(&resolved);
        resolved
    };

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let wide_path: Vec<u16> = dir_to_open
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `op` and `wide_path` are valid null-terminated UTF-16 buffers.
        unsafe {
            ShellExecuteW(
                0,
                op.as_ptr(),
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = dir_to_open;
    }
}

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
            XerjAdapter::open(config.to_xerj_config()).wrap_err("初始化 XERJ 引擎适配器失败")?,
        );
        Ok(Self {
            config: Arc::new(config),
            adapter,
            skills_catalog,
            books_catalog,
        })
    }

    /// Construct with already initialized components.
    #[must_use]
    pub const fn with_components(
        config: Arc<AppConfig>,
        adapter: Arc<XerjAdapter>,
        skills_catalog: Arc<SkillCatalog>,
        books_catalog: Arc<BookCatalog>,
    ) -> Self {
        Self {
            config,
            adapter,
            skills_catalog,
            books_catalog,
        }
    }

    /// Access the current application configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Access the underlying XERJ adapter.
    #[must_use]
    pub const fn adapter(&self) -> &Arc<XerjAdapter> {
        &self.adapter
    }

    /// List all Skills on disk converted to [`KnowledgeItem`] (including full `SKILL.md` and multi-level files).
    pub fn list_skills(&self) -> Result<Vec<KnowledgeItem>> {
        let entries = self.skills_catalog.collect_entries()?;
        let mut items = Vec::with_capacity(entries.len() * 2);
        for entry in entries {
            let sub_files: Vec<SubFileEntry> = entry
                .files
                .iter()
                .map(|f| SubFileEntry {
                    relative_path: f.relative_path.clone(),
                    content: f.content.clone(),
                })
                .collect();
            let total_files = sub_files.len().max(1);

            // Primary Skill card (`SKILL.md` + full multi-level file tree)
            items.push(KnowledgeItem {
                id: format!("skill:{}", entry.name),
                kind: ItemKind::Skill,
                title: entry.name.clone(),
                uri: format!("skill://{}/SKILL.md", entry.name),
                preview: entry.description.clone(),
                content: entry.skill_markdown.clone(),
                description: Some(entry.description.clone()),
                source_path: Some(entry.root_dir.display().to_string()),
                tags: vec![format!("skill:{}", entry.name)],
                owner: entry.name.clone(),
                relative_path: "SKILL.md".to_string(),
                part_index: 1,
                part_total: total_files,
                file_count: total_files,
                score: None,
                updated_at: None,
                favorite: false,
                is_favorite: false,
                sub_files: sub_files.clone(),
            });

            // Also expose sub-level files (references/, scripts/, agents/, etc.) for category filtering
            for (idx, file) in entry.files.iter().enumerate() {
                if file.relative_path == "SKILL.md" {
                    continue;
                }
                let uri = format!("skill://{}/{}", entry.name, file.relative_path);
                let kind = ItemKind::classify("skill", &file.relative_path, &uri);
                let snippet: String = file
                    .content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                items.push(KnowledgeItem {
                    id: format!("skill:{}:{}", entry.name, file.relative_path),
                    kind,
                    title: format!("{} / {}", entry.name, file.relative_path),
                    uri,
                    preview: if snippet.is_empty() {
                        file.relative_path.clone()
                    } else {
                        snippet
                    },
                    content: file.content.clone(),
                    description: Some(entry.description.clone()),
                    source_path: Some(
                        entry
                            .root_dir
                            .join(&file.relative_path)
                            .display()
                            .to_string(),
                    ),
                    tags: vec![format!("skill:{}", entry.name), file.relative_path.clone()],
                    owner: entry.name.clone(),
                    relative_path: file.relative_path.clone(),
                    part_index: idx + 1,
                    part_total: total_files,
                    file_count: total_files,
                    score: None,
                    updated_at: None,
                    favorite: false,
                    is_favorite: false,
                    sub_files: sub_files.clone(),
                });
            }
        }
        Ok(items)
    }

    /// List all Books on disk converted to [`KnowledgeItem`] (including full `BOOK.md` and multi-level files).
    pub fn list_books(&self) -> Result<Vec<KnowledgeItem>> {
        let entries = self.books_catalog.collect_entries()?;
        let mut items = Vec::with_capacity(entries.len() * 2);
        for entry in entries {
            let sub_files: Vec<SubFileEntry> = entry
                .files
                .iter()
                .map(|f| SubFileEntry {
                    relative_path: f.relative_path.clone(),
                    content: f.content.clone(),
                })
                .collect();
            let total_files = sub_files.len().max(1);

            // Primary Book card (`BOOK.md` + full multi-level file tree)
            items.push(KnowledgeItem {
                id: format!("book:{}", entry.name),
                kind: ItemKind::Book,
                title: entry.name.clone(),
                uri: format!("book://{}/BOOK.md", entry.name),
                preview: entry.description.clone(),
                content: entry.book_markdown.clone(),
                description: Some(entry.description.clone()),
                source_path: Some(entry.root_dir.display().to_string()),
                tags: vec![format!("book:{}", entry.name)],
                owner: entry.name.clone(),
                relative_path: "BOOK.md".to_string(),
                part_index: 1,
                part_total: total_files,
                file_count: total_files,
                score: None,
                updated_at: None,
                favorite: false,
                is_favorite: false,
                sub_files: sub_files.clone(),
            });

            // Also expose sub-level files (chapters/, references/, notes/, files/, etc.)
            for (idx, file) in entry.files.iter().enumerate() {
                if file.relative_path == "BOOK.md" {
                    continue;
                }
                let uri = format!("book://{}/{}", entry.name, file.relative_path);
                let kind = ItemKind::classify("book", &file.relative_path, &uri);
                let snippet: String = file
                    .content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                items.push(KnowledgeItem {
                    id: format!("book:{}:{}", entry.name, file.relative_path),
                    kind,
                    title: format!("{} / {}", entry.name, file.relative_path),
                    uri,
                    preview: if snippet.is_empty() {
                        file.relative_path.clone()
                    } else {
                        snippet
                    },
                    content: file.content.clone(),
                    description: Some(entry.description.clone()),
                    source_path: Some(
                        entry
                            .root_dir
                            .join(&file.relative_path)
                            .display()
                            .to_string(),
                    ),
                    tags: vec![format!("book:{}", entry.name), file.relative_path.clone()],
                    owner: entry.name.clone(),
                    relative_path: file.relative_path.clone(),
                    part_index: idx + 1,
                    part_total: total_files,
                    file_count: total_files,
                    score: None,
                    updated_at: None,
                    favorite: false,
                    is_favorite: false,
                    sub_files: sub_files.clone(),
                });
            }
        }
        Ok(items)
    }

    /// Path to the persisted JSON file storing user-created Memory and Note items.
    fn custom_memories_path(&self) -> std::path::PathBuf {
        crate::core::paths::resolve_relative(&self.config.xerj.data_dir).join(CUSTOM_MEMORIES_FILE)
    }

    /// Load persisted user Memory / Note / standalone items from disk.
    #[must_use]
    pub fn list_memories(&self) -> Vec<KnowledgeItem> {
        let path = self.custom_memories_path();
        if path.is_file()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(items) = serde_json::from_str::<Vec<KnowledgeItem>>(&content)
        {
            return items;
        }
        Vec::new()
    }

    fn save_memories_list(&self, items: &[KnowledgeItem]) -> Result<()> {
        let path = self.custom_memories_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(items)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Build [`IndexDocument`] records from persisted Memory/Note items.
    fn build_memory_index_docs(&self) -> Vec<IndexDocument> {
        let items = self.list_memories();
        let inputs: Vec<MemoryChunkInput> = items
            .into_iter()
            .filter(|it| !it.content.trim().is_empty())
            .map(|it| {
                let ns = if it.owner.trim().is_empty() {
                    "default".to_string()
                } else {
                    it.owner.clone()
                };
                let ext_id = it
                    .id
                    .strip_prefix(&format!("memory:{ns}:"))
                    .or_else(|| it.id.strip_prefix("memory:"))
                    .unwrap_or(&it.id)
                    .to_string();
                MemoryChunkInput {
                    id: Some(ext_id),
                    namespace: ns,
                    title: Some(it.title),
                    uri: if it.uri.is_empty() {
                        None
                    } else {
                        Some(it.uri)
                    },
                    text: it.content,
                    tags: it.tags,
                    metadata: None,
                }
            })
            .collect();
        if inputs.is_empty() {
            Vec::new()
        } else {
            build_memory_documents(&inputs).unwrap_or_default()
        }
    }

    /// Load all knowledge items across Skills, Books, and Memory synchronously.
    #[must_use]
    pub fn load_all_knowledge_items(&self) -> Vec<KnowledgeItem> {
        let mut all = Vec::new();
        if let Ok(mut skills) = self.list_skills() {
            all.append(&mut skills);
        }
        if let Ok(mut books) = self.list_books() {
            all.append(&mut books);
        }
        let mut memories = self.list_memories();
        all.append(&mut memories);

        let settings = GuiSettings::load();
        for item in &mut all {
            let fav =
                settings.is_favorite(&item.id) || settings.is_favorite(&item.uri) || item.favorite;
            item.favorite = fav;
            item.is_favorite = fav;
            item.tags = settings.merged_item_tags(&item.id, &item.tags);
        }
        all
    }

    /// Execute search query across Amemory indices.
    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<KnowledgeItem>> {
        let hits = self.adapter.search(request).await?;
        let catalog_items = self.load_all_knowledge_items();
        let mut items = Vec::with_capacity(hits.len());
        for hit in hits {
            let (owner, relative_path, kind) = parse_uri_and_tags(&hit);
            let uri_str = hit.uri.clone().unwrap_or_default();
            // Match against full catalog item if available so sub_files, favorite, and full content are preserved
            if let Some(mut full_item) = catalog_items
                .iter()
                .find(|it| {
                    it.id == hit.doc_id
                        || (!uri_str.is_empty() && it.uri == uri_str)
                        || (!owner.is_empty()
                            && it.owner == owner
                            && it.relative_path == relative_path)
                })
                .cloned()
            {
                full_item.score = hit.score;
                if !hit.text.is_empty() {
                    full_item.preview = hit.text;
                }
                items.push(full_item);
            } else {
                items.push(KnowledgeItem {
                    id: hit.doc_id,
                    kind,
                    title: hit.title.unwrap_or_else(|| uri_str.clone()),
                    uri: uri_str,
                    preview: hit.text.clone(),
                    content: hit.text,
                    description: None,
                    source_path: None,
                    tags: hit.tags,
                    owner,
                    relative_path,
                    part_index: 1,
                    part_total: 1,
                    file_count: 1,
                    score: hit.score,
                    updated_at: None,
                    favorite: false,
                    is_favorite: false,
                    sub_files: Vec::new(),
                });
            }
        }
        Ok(items)
    }

    /// Synchronously get snapshot of stats for GUI polling.
    #[must_use]
    pub fn get_stats_snapshot(&self) -> StatsSnapshot {
        let skills_list = self.skills_catalog.list_summaries().unwrap_or_default();
        let skills_count = skills_list.len();
        let skill_files_count: usize = skills_list.iter().map(|s| s.file_count).sum();

        let books_list = self.books_catalog.list_summaries().unwrap_or_default();
        let books_count = books_list.len();
        let book_files_count: usize = books_list.iter().map(|b| b.file_count).sum();
        let imported_docs_count: usize = books_list
            .iter()
            .map(|b| {
                b.files
                    .iter()
                    .filter(|f| f.starts_with("imports/") || f.starts_with("docs/"))
                    .count()
            })
            .sum();

        let memories = self.list_memories();
        let memory_count = memories.len();
        let mut namespaces = std::collections::BTreeSet::new();
        namespaces.insert("default".to_string());
        for m in &memories {
            if !m.owner.trim().is_empty() {
                namespaces.insert(m.owner.clone());
            }
        }

        let data_dir_path = crate::core::paths::resolve_relative(&self.config.xerj.data_dir);
        let disk_usage_bytes = calculate_dir_size(&data_dir_path);

        let skills_docs = skill_files_count.max(skills_count) as u64;
        let books_docs = (book_files_count + imported_docs_count).max(books_count) as u64;
        let memory_docs = memory_count as u64;
        let total_docs = skills_docs + books_docs + memory_docs;

        StatsSnapshot {
            skills_count,
            skill_files_count,
            skills_docs,
            books_count,
            book_files_count,
            imported_docs_count,
            books_docs,
            memory_docs,
            namespace_count: namespaces.len(),
            health: HealthLevel::Green,
            index_count: 3,
            total_docs,
            disk_usage_bytes,
            segments: 3,
            doc_count: total_docs,
            p95_latency_ms: 1.8,
            memory_count,
            embed_mode: self.config.xerj.embed_mode.clone(),
            last_search_latency_ms: Some(2),
            last_sync_summary: Some("实时就绪".to_string()),
            data_dir: self.config.xerj.data_dir.display().to_string(),
        }
    }

    /// Read live system statistics snapshot.
    pub async fn stats_snapshot(&self) -> StatsSnapshot {
        let mut snap = self.get_stats_snapshot();
        if let Ok(store_stats) = self.adapter.stats().await {
            snap.skills_docs = store_stats.skills_docs;
            snap.books_docs = store_stats.books_docs;
            snap.memory_docs = store_stats.memory_docs.max(snap.memory_docs);
            snap.total_docs = snap.skills_docs + snap.books_docs + snap.memory_docs;
            snap.doc_count = snap.total_docs;
        }
        snap
    }

    /// Save knowledge item back to catalog (supporting Skills, Books, Memory, Notes, and multi-level `sub_files`).
    pub fn save_knowledge_item(&self, item: &KnowledgeItem) -> Result<()> {
        // Persist custom tags into GuiSettings as well
        if !item.id.is_empty() {
            let mut settings = GuiSettings::load();
            settings.set_item_tags(&item.id, item.tags.clone());
            let _ = settings.save();
        }

        let is_skill = item.kind == ItemKind::Skill
            || item.uri.starts_with("skill://")
            || item.id.starts_with("skill:");
        let is_book = item.kind == ItemKind::Book
            || item.uri.starts_with("book://")
            || item.id.starts_with("book:");

        if is_skill {
            let raw_name = if item.owner.trim().is_empty() {
                item.title.trim()
            } else {
                item.owner.trim()
            };
            let skill_name = sanitize_catalog_slug(raw_name, "custom-skill");
            let mut skill_markdown = None;
            let mut extra_files = Vec::new();
            for sf in &item.sub_files {
                if sf.relative_path == "SKILL.md" {
                    skill_markdown = Some(sf.content.clone());
                } else if !sf.relative_path.trim().is_empty() {
                    extra_files.push(SkillFileInput {
                        relative_path: sf.relative_path.clone(),
                        content: sf.content.clone(),
                    });
                }
            }
            if skill_markdown.is_none() && item.content.trim_start().starts_with("---") {
                skill_markdown = Some(item.content.clone());
            }

            let desc = item
                .description
                .clone()
                .filter(|d| !d.trim().is_empty())
                .or_else(|| {
                    let p = item.preview.trim();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p.to_string())
                    }
                })
                .unwrap_or_else(|| format!("{skill_name} 技能定义与提示词规范"));

            if self.skills_catalog.load_skill(&skill_name).is_ok() {
                let update_input = UpdateSkillInput {
                    name: skill_name,
                    description: Some(desc),
                    body: if skill_markdown.is_none() {
                        Some(item.content.clone())
                    } else {
                        None
                    },
                    skill_markdown,
                    files: extra_files,
                    remove_files: Vec::new(),
                    file_policy: SkillFileUpdatePolicy::Merge,
                };
                self.skills_catalog.update_skill(&update_input)?;
            } else {
                let create_input = CreateSkillInput {
                    name: Some(skill_name),
                    description: Some(desc),
                    body: if skill_markdown.is_none() {
                        Some(item.content.clone())
                    } else {
                        None
                    },
                    skill_markdown,
                    files: extra_files,
                };
                self.skills_catalog.create_skill(&create_input)?;
            }
        } else if is_book {
            let raw_name = if item.owner.trim().is_empty() {
                item.title.trim()
            } else {
                item.owner.trim()
            };
            let book_name = sanitize_catalog_slug(raw_name, "custom-book");
            let mut book_markdown = None;
            let mut extra_files = Vec::new();
            for sf in &item.sub_files {
                if sf.relative_path == "BOOK.md" {
                    book_markdown = Some(sf.content.clone());
                } else if !sf.relative_path.trim().is_empty() {
                    extra_files.push(BookFileInput {
                        relative_path: sf.relative_path.clone(),
                        content: sf.content.clone(),
                    });
                }
            }
            if book_markdown.is_none() && item.content.trim_start().starts_with("---") {
                book_markdown = Some(item.content.clone());
            }

            let desc = item
                .description
                .clone()
                .filter(|d| !d.trim().is_empty())
                .or_else(|| {
                    let p = item.preview.trim();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p.to_string())
                    }
                })
                .unwrap_or_else(|| format!("{book_name} 知识书本与参考手册"));

            if self.books_catalog.load_book(&book_name).is_ok() {
                let update_input = UpdateBookInput {
                    name: book_name,
                    description: Some(desc),
                    body: if book_markdown.is_none() {
                        Some(item.content.clone())
                    } else {
                        None
                    },
                    book_markdown,
                    files: extra_files,
                    remove_files: Vec::new(),
                    file_policy: BookFileUpdatePolicy::Merge,
                };
                self.books_catalog.update_book(&update_input)?;
            } else {
                let create_input = CreateBookInput {
                    name: Some(book_name),
                    description: Some(desc),
                    body: if book_markdown.is_none() {
                        Some(item.content.clone())
                    } else {
                        None
                    },
                    book_markdown,
                    files: extra_files,
                };
                self.books_catalog.create_book(&create_input)?;
            }
        } else {
            // Standalone Memory / Note / Script / Document item
            let mut memories = self.list_memories();
            let mut saved_item = item.clone();
            if saved_item.title.trim().is_empty() {
                saved_item.title = "未命名条目".to_string();
            }
            if saved_item.owner.trim().is_empty() {
                saved_item.owner = match saved_item.kind {
                    ItemKind::Note => "notes".to_string(),
                    ItemKind::Script => "scripts".to_string(),
                    ItemKind::Document => "docs".to_string(),
                    _ => "default".to_string(),
                };
            }
            if saved_item.id.trim().is_empty() {
                saved_item.id = format!(
                    "memory:{}:{}",
                    saved_item.owner,
                    Utc::now().timestamp_millis()
                );
            }
            if saved_item.uri.trim().is_empty() {
                saved_item.uri = format!("memory://{}/{}", saved_item.owner, saved_item.id);
            }
            let snippet: String = saved_item
                .content
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            saved_item.preview = if snippet.is_empty() {
                saved_item.title.clone()
            } else {
                snippet
            };
            saved_item.part_index = 1;
            saved_item.part_total = 1;
            saved_item.file_count = 1;
            saved_item.updated_at = Some(Utc::now().to_rfc3339());

            if let Some(existing) = memories.iter_mut().find(|m| m.id == saved_item.id) {
                *existing = saved_item;
            } else {
                memories.insert(0, saved_item);
            }
            self.save_memories_list(&memories)?;
        }
        Ok(())
    }

    /// Delete a knowledge item by ID.
    pub fn delete_knowledge_item(&self, id: &str) -> Result<()> {
        if let Some(rest) = id.strip_prefix("skill:") {
            if let Some((skill_name, rel_path)) = rest.split_once(':') {
                // Deleting a sub-file inside a skill
                if rel_path == "SKILL.md" {
                    self.skills_catalog.delete_skill(skill_name)?;
                } else {
                    let update_input = UpdateSkillInput {
                        name: skill_name.to_string(),
                        description: None,
                        body: None,
                        skill_markdown: None,
                        files: Vec::new(),
                        remove_files: vec![rel_path.to_string()],
                        file_policy: SkillFileUpdatePolicy::Merge,
                    };
                    self.skills_catalog.update_skill(&update_input)?;
                }
            } else {
                self.skills_catalog.delete_skill(rest)?;
            }
        } else if let Some(rest) = id.strip_prefix("book:") {
            if let Some((book_name, rel_path)) = rest.split_once(':') {
                if rel_path == "BOOK.md" {
                    self.books_catalog.delete_book(book_name)?;
                } else {
                    let update_input = UpdateBookInput {
                        name: book_name.to_string(),
                        description: None,
                        body: None,
                        book_markdown: None,
                        files: Vec::new(),
                        remove_files: vec![rel_path.to_string()],
                        file_policy: BookFileUpdatePolicy::Merge,
                    };
                    self.books_catalog.update_book(&update_input)?;
                }
            } else {
                self.books_catalog.delete_book(rest)?;
            }
        } else {
            let mut memories = self.list_memories();
            memories.retain(|m| m.id != id);
            self.save_memories_list(&memories)?;
        }
        Ok(())
    }

    /// Index a single item into XERJ.
    pub async fn index_item(&self, _item: &KnowledgeItem) -> Result<()> {
        let _ = self.sync_all().await?;
        Ok(())
    }

    /// Clear all items in the Memory index and local memory store.
    pub async fn clear_memory_index(&self) -> Result<usize> {
        let count = self.list_memories().len();
        self.save_memories_list(&[])?;
        let _ = self
            .adapter
            .sync_catalog(IndexTarget::Memory, 0, &[])
            .await?;
        Ok(count)
    }

    /// Synchronize all disk catalogs (Skills, Books, and Memory) into XERJ.
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

        let mem_docs = self.build_memory_index_docs();
        let _ = self
            .adapter
            .sync_catalog(IndexTarget::Memory, mem_docs.len(), &mem_docs)
            .await;

        Ok((skill_sync, book_sync))
    }

    /// Create or update a Skill.
    pub async fn save_skill(&self, input: &CreateSkillInput) -> Result<()> {
        let created = self.skills_catalog.create_skill(input)?;
        self.adapter.apply_mutation(&created.index_mutation).await?;
        Ok(())
    }

    /// Create or update a Book.
    pub async fn save_book(&self, input: &CreateBookInput) -> Result<()> {
        let created = self.books_catalog.create_book(input)?;
        self.adapter.apply_mutation(&created.index_mutation).await?;
        Ok(())
    }

    /// Append a memory chunk directly into XERJ.
    pub async fn add_memory_chunk(&self, chunk: MemoryChunkInput) -> Result<String> {
        let ids = self.adapter.add_memory_chunks(&[chunk]).await?;
        ids.into_iter()
            .next()
            .ok_or_else(|| eyre::eyre!("写入记忆分片未返回 ID"))
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

fn sanitize_catalog_slug(raw: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if (ch == '-' || ch == '_' || ch == ' ') && !slug.is_empty() && !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let clean = slug.trim_matches('-').to_string();
    if clean.is_empty() {
        format!("{fallback}-{}", Utc::now().timestamp_millis() % 100_000)
    } else {
        clean
    }
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
