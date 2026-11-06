//! Background synchronization service between catalog directories and XERJ.

use std::sync::Arc;

use domain::memory::SyncReport;
use eyre::Result;
use tracing::{error, info};

use super::amemory_runtime::AmemoryRuntime;

/// Synchronize all skills and books in the background.
pub async fn run_catalog_sync(
    runtime: Arc<AmemoryRuntime>,
) -> Result<(SyncReport, SyncReport)> {
    info!("Starting catalog synchronization...");
    match runtime.sync_all().await {
        Ok((skill_report, book_report)) => {
            info!(
                "Catalog sync completed: skills (scanned={}, indexed={}, unchanged={}, deleted={}), books (scanned={}, indexed={}, unchanged={}, deleted={})",
                skill_report.scanned_items,
                skill_report.indexed_documents,
                skill_report.unchanged_documents,
                skill_report.deleted_documents,
                book_report.scanned_items,
                book_report.indexed_documents,
                book_report.unchanged_documents,
                book_report.deleted_documents
            );
            Ok((skill_report, book_report))
        }
        Err(err) => {
            error!("Catalog sync failed: {err:#}");
            Err(err)
        }
    }
}
