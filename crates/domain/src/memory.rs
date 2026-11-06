//! Domain models and deterministic chunking for Memory, Search, and Index synchronization.

use blake3::Hasher;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::DomainError;

/// Default maximum character length per indexed chunk part.
pub const DEFAULT_MAX_PART_CHARS: usize = 1_800;

/// Default namespace for general memory chunks when none is specified.
pub const DEFAULT_MEMORY_NAMESPACE: &str = "default";

/// Logical index target inside the search engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexTarget {
    /// Dedicated index for Skill documents (`amemory-skills-v1`).
    Skills,
    /// Dedicated index for Book and imported Book documents (`amemory-books-v1`).
    Books,
    /// Dedicated index for general Memory chunks (`amemory-memory-v1`).
    Memory,
}

impl IndexTarget {
    /// Return the canonical string label for this index target.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Books => "books",
            Self::Memory => "memory",
        }
    }
}

/// Retrieval mode for unified search requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Full-text BM25 lexical retrieval.
    Lexical,
    /// Server-side semantic embedding retrieval.
    Semantic,
    /// Reciprocal-rank fusion combining lexical and semantic retrieval.
    #[default]
    Hybrid,
}

/// Storage-agnostic document representation ready for indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocument {
    /// Deterministic document identifier (`_id`).
    pub doc_id: String,
    /// Target logical index.
    pub target: IndexTarget,
    /// Human-readable document title.
    pub title: String,
    /// Logical URI (`skill://...`, `book://...`, `memory://...`).
    pub uri: String,
    /// Searchable body text of the chunk.
    pub text: String,
    /// Normalized metadata tags (`kind:...`, `skill:...`, `book:...`, etc.).
    pub tags: Vec<String>,
    /// Source category (`skill`, `book`, `book_document`, `memory`).
    pub source_kind: String,
    /// Owning entity name (`skill_name`, `book_name`, or `namespace`).
    pub owner_name: String,
    /// Relative path within the owning entity.
    pub relative_path: String,
    /// 1-based part number when split across multiple chunks.
    pub part_index: usize,
    /// Total number of parts for the logical file/chunk.
    pub part_total: usize,
    /// Hex-encoded BLAKE3 digest of the canonical source content.
    pub content_hash: String,
    /// RFC 3339 UTC timestamp of the document creation or update.
    pub updated_at: String,
    /// Extra domain-specific fields stored in the document source.
    pub extra: Map<String, Value>,
}

/// Batch of index upserts and deletions scoped to an index target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexMutation {
    /// Target index for this mutation.
    pub target: IndexTarget,
    /// Optional document ID prefix for the owning entity (e.g. `skill:rust:`).
    pub owner_prefix: Option<String>,
    /// Documents to insert or overwrite.
    pub upserts: Vec<IndexDocument>,
    /// Explicit document IDs to remove.
    pub delete_doc_ids: Vec<String>,
}

/// Summary report returned after synchronizing a catalog with the index store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncReport {
    /// Number of catalog items (Skills or Books) scanned on disk.
    pub scanned_items: usize,
    /// Number of chunk documents written or updated in the index.
    pub indexed_documents: usize,
    /// Number of chunk documents skipped because their content hash was unchanged.
    pub unchanged_documents: usize,
    /// Number of stale chunk documents deleted from the index.
    pub deleted_documents: usize,
    /// Human-readable summary message.
    pub message: String,
}

/// Aggregate document counts across the three Amemory indices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreStats {
    /// Live document count in the Skills index.
    pub skills_docs: u64,
    /// Live document count in the Books index.
    pub books_docs: u64,
    /// Live document count in the Memory index.
    pub memory_docs: u64,
    /// Underlying XERJ data directory path.
    pub data_dir: String,
}

/// Domain-level search query parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query string (required, non-empty).
    pub query: String,
    /// Maximum number of hits to return (`1..=100`).
    pub limit: usize,
    /// Maximum characters in the returned text snippet (`80..=4000`).
    pub snippet_chars: usize,
    /// Retrieval strategy (`Lexical`, `Semantic`, or `Hybrid`).
    pub mode: SearchMode,
    /// Optional exact URI filter.
    pub uri: Option<String>,
    /// Optional URI prefix scope filter (e.g. `skill://` or `book://`).
    pub scope: Option<String>,
    /// Exact URIs to exclude from results.
    pub exclude_uris: Vec<String>,
    /// URI prefixes to exclude from results.
    pub exclude_scopes: Vec<String>,
    /// Normalized `key:value` or tag strings that hits must carry.
    pub tag_filters: Vec<String>,
}

impl SearchRequest {
    /// Validate and normalize search request bounds.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::Validation`] if `query` is blank.
    pub fn validated(mut self) -> Result<Self, DomainError> {
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("query 不能为空".to_string()));
        }
        self.query = trimmed.to_string();
        self.limit = self.limit.clamp(1, 100);
        self.snippet_chars = self.snippet_chars.clamp(80, 4_000);
        self.uri = self
            .uri
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.scope = self
            .scope
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.exclude_uris = self
            .exclude_uris
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.exclude_scopes = self
            .exclude_scopes
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.tag_filters = self
            .tag_filters
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(self)
    }
}

/// Normalized search hit returned from the index adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    /// Document identifier (`_id`).
    pub doc_id: String,
    /// Concrete index name where the hit originated.
    pub index: String,
    /// Relevance score assigned by the engine.
    pub score: Option<f32>,
    /// Optional document title.
    pub title: Option<String>,
    /// Optional logical URI.
    pub uri: Option<String>,
    /// Snippet or full text of the matching chunk.
    pub text: String,
    /// Document tags.
    pub tags: Vec<String>,
    /// Additional document metadata.
    pub metadata: Value,
}

/// Input payload for writing a general memory chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryChunkInput {
    /// Optional caller-provided external identifier; defaults to content hash when absent.
    pub id: Option<String>,
    /// Memory namespace (defaults to `"default"` when blank).
    pub namespace: String,
    /// Optional human-readable title.
    pub title: Option<String>,
    /// Optional logical URI (`memory://...`).
    pub uri: Option<String>,
    /// Chunk text content.
    pub text: String,
    /// Optional tags.
    pub tags: Vec<String>,
    /// Optional structured JSON metadata.
    pub metadata: Option<Value>,
}

/// Compute the lowercase hexadecimal BLAKE3 digest of `bytes`.
#[must_use]
pub fn content_hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

/// Build the deterministic document ID for a Skill file chunk.
#[must_use]
pub fn skill_doc_id(skill_name: &str, relative_path: &str, part_index: usize) -> String {
    format!("skill:{skill_name}:{relative_path}:{part_index}")
}

/// Build the document ID prefix for all chunks belonging to `skill_name`.
#[must_use]
pub fn skill_doc_prefix(skill_name: &str) -> String {
    format!("skill:{skill_name}:")
}

/// Build the deterministic document ID for a Book file chunk.
#[must_use]
pub fn book_doc_id(book_name: &str, relative_path: &str, part_index: usize) -> String {
    format!("book:{book_name}:{relative_path}:{part_index}")
}

/// Build the deterministic document ID for an imported Book document chunk.
#[must_use]
pub fn book_document_doc_id(book_name: &str, document_hash: &str, part_index: usize) -> String {
    format!("book-document:{book_name}:{document_hash}:{part_index}")
}

/// Build the document ID prefix for all regular chunks belonging to `book_name`.
#[must_use]
pub fn book_doc_prefix(book_name: &str) -> String {
    format!("book:{book_name}:")
}

/// Build the document ID prefix for all imported document chunks belonging to `book_name`.
#[must_use]
pub fn book_document_doc_prefix(book_name: &str) -> String {
    format!("book-document:{book_name}:")
}

/// Build the deterministic document ID for a general memory chunk.
#[must_use]
pub fn memory_doc_id(namespace: &str, external_id: &str) -> String {
    format!("memory:{namespace}:{external_id}")
}

/// Convert a batch of [`MemoryChunkInput`] items into validated [`IndexDocument`] records.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `inputs` is empty or any chunk has blank `text`.
pub fn build_memory_documents(
    inputs: &[MemoryChunkInput],
) -> Result<Vec<IndexDocument>, DomainError> {
    if inputs.is_empty() {
        return Err(DomainError::Validation(
            "chunks 不能为空，至少需要写入一条内容".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let mut documents = Vec::with_capacity(inputs.len());

    for (idx, item) in inputs.iter().enumerate() {
        let text = item.text.trim();
        if text.is_empty() {
            return Err(DomainError::Validation(format!(
                "第 {} 条 chunk 的 text 不能为空",
                idx + 1
            )));
        }

        let namespace = {
            let trimmed = item.namespace.trim();
            if trimmed.is_empty() {
                DEFAULT_MEMORY_NAMESPACE.to_string()
            } else {
                trimmed.to_string()
            }
        };

        let hash = content_hash_hex(text.as_bytes());
        let external_id = item
            .id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(|| hash.clone(), str::to_string);
        let doc_id = memory_doc_id(&namespace, &external_id);

        let title = item
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(|| format!("Memory #{external_id}"), str::to_string);

        let uri = item
            .uri
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(
                || format!("memory://{namespace}/{external_id}"),
                str::to_string,
            );

        let mut tags = vec!["kind:memory".to_string(), format!("namespace:{namespace}")];
        for tag in &item.tags {
            let trimmed = tag.trim();
            if !trimmed.is_empty() && !tags.iter().any(|existing| existing == trimmed) {
                tags.push(trimmed.to_string());
            }
        }

        let mut extra = Map::new();
        extra.insert("namespace".to_string(), Value::String(namespace.clone()));
        if let Some(meta) = &item.metadata {
            extra.insert("metadata".to_string(), meta.clone());
        }

        documents.push(IndexDocument {
            doc_id,
            target: IndexTarget::Memory,
            title,
            uri,
            text: text.to_string(),
            tags,
            source_kind: "memory".to_string(),
            owner_name: namespace,
            relative_path: external_id,
            part_index: 1,
            part_total: 1,
            content_hash: hash,
            updated_at: now.clone(),
            extra,
        });
    }

    Ok(documents)
}

/// Split a long text document into chunks of at most `max_chars` characters,
/// preferring paragraph and line boundaries when possible.
#[must_use]
pub fn split_text_into_parts(text: &str, max_chars: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let cap = max_chars.max(64);
    if trimmed.chars().count() <= cap {
        return vec![trimmed.to_string()];
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for paragraph in trimmed.split("\n\n") {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }

        let paragraph_chars = paragraph.chars().count();
        if paragraph_chars > cap {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
                current_chars = 0;
            }
            parts.extend(split_large_block(paragraph, cap));
            continue;
        }

        let separator_chars = if current.is_empty() { 0 } else { 2 };
        if current_chars + separator_chars + paragraph_chars > cap && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
            current_chars = 0;
        }

        if !current.is_empty() {
            current.push_str("\n\n");
            current_chars += 2;
        }
        current.push_str(paragraph);
        current_chars += paragraph_chars;
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn split_large_block(block: &str, max_chars: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for ch in block.chars() {
        current.push(ch);
        current_chars += 1;
        if current_chars >= max_chars {
            let chunk = current.trim().to_string();
            if !chunk.is_empty() {
                parts.push(chunk);
            }
            current.clear();
            current_chars = 0;
        }
    }

    let tail = current.trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_ids_match_specification() {
        assert_eq!(
            skill_doc_id("rust-expert", "SKILL.md", 1),
            "skill:rust-expert:SKILL.md:1"
        );
        assert_eq!(
            book_doc_id("clean-code", "BOOK.md", 2),
            "book:clean-code:BOOK.md:2"
        );
        assert_eq!(
            book_document_doc_id("clean-code", "abcd1234", 1),
            "book-document:clean-code:abcd1234:1"
        );
        assert_eq!(memory_doc_id("default", "note-1"), "memory:default:note-1");
    }

    #[test]
    fn split_text_respects_max_chars() {
        let text = format!("{}\n\n{}", "a".repeat(120), "b".repeat(120));
        let parts = split_text_into_parts(&text, 150);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 120);
        assert_eq!(parts[1].len(), 120);
    }
}
