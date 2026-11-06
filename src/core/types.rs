//! Framework-agnostic data models displayed by the GUI.

use serde::{Deserialize, Serialize};

/// Knowledge category shown in the type filter bar and on card badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// A Skill (`SKILL.md`).
    Skill,
    /// A Book (`BOOK.md`) or one of its chapters.
    Book,
    /// An imported document stored inside a Book (`files/imports/...`).
    Document,
    /// A general memory chunk (`memory://...`).
    Memory,
    /// A script or code file (`scripts/...`).
    Script,
    /// A reference or note file (`references/...`, `notes/...`).
    Note,
}

impl ItemKind {
    /// All kinds in their default filter-bar order.
    pub const ALL: [Self; 6] = [
        Self::Skill,
        Self::Book,
        Self::Document,
        Self::Memory,
        Self::Script,
        Self::Note,
    ];

    /// Stable identifier used in persisted settings.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Book => "book",
            Self::Document => "document",
            Self::Memory => "memory",
            Self::Script => "script",
            Self::Note => "note",
        }
    }

    /// Parse a persisted identifier.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }

    /// Classify an indexed chunk by its `source_kind` and relative path.
    #[must_use]
    pub fn classify(source_kind: &str, relative_path: &str, uri: &str) -> Self {
        let path = relative_path.replace('\\', "/");
        if source_kind == "memory" || uri.starts_with("memory://") {
            return Self::Memory;
        }
        if source_kind == "book_document" || path.starts_with("files/") {
            return Self::Document;
        }
        if path.starts_with("scripts/") {
            return Self::Script;
        }
        if path.starts_with("references/")
            || path.starts_with("notes/")
            || path.starts_with("summaries/")
        {
            return Self::Note;
        }
        if source_kind.starts_with("book") || uri.starts_with("book://") {
            Self::Book
        } else {
            Self::Skill
        }
    }
}

/// A single row in the knowledge list (Clippi's "clipboard card" equivalent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeItem {
    /// Unique document or entity identifier.
    pub id: String,
    /// Category used for icons and filtering.
    pub kind: ItemKind,
    /// Display title.
    pub title: String,
    /// Logical URI (`skill://`, `book://`, `memory://`).
    pub uri: String,
    /// Text preview or search snippet.
    pub preview: String,
    /// Tags attached to the item.
    pub tags: Vec<String>,
    /// Owning Skill/Book name or memory namespace.
    pub owner: String,
    /// Relative path inside the owner, when applicable.
    pub relative_path: String,
    /// 1-based chunk index.
    pub part_index: usize,
    /// Total chunk count.
    pub part_total: usize,
    /// Number of files belonging to the owner (Skill/Book entries).
    pub file_count: usize,
    /// Relevance score for search hits.
    pub score: Option<f32>,
    /// RFC 3339 update timestamp, when known.
    pub updated_at: Option<String>,
    /// Whether the user marked this item as favorite.
    pub favorite: bool,
}

impl KnowledgeItem {
    /// Whether this item maps to a whole Skill or Book entity (editable/deletable on disk).
    #[must_use]
    pub fn is_catalog_entry(&self) -> bool {
        matches!(self.kind, ItemKind::Skill | ItemKind::Book) && self.relative_path.is_empty()
    }
}

/// XERJ engine health level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HealthLevel {
    /// All indices open and searchable.
    Green,
    /// Partially degraded.
    Yellow,
    /// Engine unavailable.
    Red,
    /// Not yet measured.
    #[default]
    Unknown,
}

/// Aggregated statistics shown on the home dashboard.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatsSnapshot {
    /// Number of Skills on disk.
    pub skills_count: usize,
    /// Number of files across all Skills.
    pub skill_files_count: usize,
    /// Indexed Skill chunks.
    pub skills_docs: u64,
    /// Number of Books on disk.
    pub books_count: usize,
    /// Number of text files across all Books.
    pub book_files_count: usize,
    /// Number of imported documents across all Books.
    pub imported_docs_count: usize,
    /// Indexed Book chunks.
    pub books_docs: u64,
    /// Indexed Memory chunks.
    pub memory_docs: u64,
    /// Distinct memory namespaces observed.
    pub namespace_count: usize,
    /// XERJ engine health.
    pub health: HealthLevel,
    /// Number of Amemory indices.
    pub index_count: usize,
    /// Total indexed documents.
    pub total_docs: u64,
    /// On-disk size of the XERJ data directory in bytes.
    pub disk_usage_bytes: u64,
    /// Current embedding mode.
    pub embed_mode: String,
    /// Latency of the most recent search in milliseconds.
    pub last_search_latency_ms: Option<u64>,
    /// Human-readable summary of the last sync.
    pub last_sync_summary: Option<String>,
    /// XERJ data directory path.
    pub data_dir: String,
}

impl StatsSnapshot {
    /// Disk usage formatted as `KB`/`MB`.
    #[must_use]
    pub fn disk_usage_label(&self) -> String {
        format_bytes(self.disk_usage_bytes)
    }
}

/// Format a byte count as a compact human-readable string.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    #[allow(clippy::cast_precision_loss)]
    let b = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", b / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", b / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", b / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_maps_paths_to_kinds() {
        assert_eq!(ItemKind::classify("skill", "SKILL.md", "skill://a/SKILL.md"), ItemKind::Skill);
        assert_eq!(ItemKind::classify("skill", "scripts/run.sh", ""), ItemKind::Script);
        assert_eq!(ItemKind::classify("book", "notes/a.md", "book://b"), ItemKind::Note);
        assert_eq!(ItemKind::classify("book_document", "files/x.pdf", ""), ItemKind::Document);
        assert_eq!(ItemKind::classify("memory", "", "memory://default/1"), ItemKind::Memory);
        assert_eq!(ItemKind::classify("book", "BOOK.md", "book://b"), ItemKind::Book);
    }

    #[test]
    fn format_bytes_uses_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
