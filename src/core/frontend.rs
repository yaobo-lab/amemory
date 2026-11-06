//! Event bus and message types between frontend UI and background services.

use std::path::PathBuf;

use super::filters::FilterCriteria;
use super::types::{ItemKind, KnowledgeItem};
use crate::config::AppConfig;

/// Events sent from the GUI frontend to background services.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Search query string changed.
    SearchQuery(String),
    /// Filter criteria changed.
    Filter(FilterCriteria),
    /// Copy item content to clipboard.
    CopyItem(KnowledgeItem),
    /// Copy item URI to clipboard.
    CopyUri(KnowledgeItem),
    /// Request to edit an item.
    EditItem(KnowledgeItem),
    /// Delete an item by ID.
    DeleteItem {
        /// Target item ID.
        id: String,
        /// Item category.
        kind: ItemKind,
        /// Owning entity.
        owner: String,
    },
    /// Toggle favorite status of an item.
    ToggleFavorite(String),
    /// Trigger catalog to XERJ index synchronization.
    TriggerSync,
    /// Trigger full index rebuild.
    TriggerRebuild,
    /// Clear all memory chunks.
    ClearMemory,
    /// Import local document into a book.
    ImportDocument {
        /// Target book name.
        book_name: String,
        /// Source file path.
        file_path: PathBuf,
    },
    /// Save application configuration to `etc/config.toml`.
    SaveConfig(AppConfig),
    /// Request update check.
    CheckUpdate,
    /// Request update download.
    DownloadUpdate,
    /// Restart application to apply update.
    RestartApp,
    /// Request flush of XERJ engine.
    FlushEngine,
}
