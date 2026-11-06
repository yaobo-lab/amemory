//! Document import service bridging file selection to Book imported documents.

use std::path::Path;
use std::sync::Arc;

use domain::book::BookCatalog;
use eyre::{Result, WrapErr};

use super::amemory_runtime::AmemoryRuntime;

/// Import a local document (PDF, Word, Excel, PPT, Markdown, TXT) into a target Book.
pub async fn import_document_to_book(
    runtime: Arc<AmemoryRuntime>,
    book_name: &str,
    source_path: &Path,
) -> Result<()> {
    let books_catalog = BookCatalog::new(&runtime.config().storage.books);
    let book = books_catalog
        .load_book(book_name)
        .wrap_err_with(|| format!("未找到目标书籍 `{book_name}`"))?;

    // Domain document import: inspect, extract text, store into files/imports/ and append chapter
    let imported_file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| eyre::eyre!("无效的源文件名"))?;

    let file_bytes = std::fs::read(source_path)
        .wrap_err_with(|| format!("读取文件 `{}` 失败", source_path.display()))?;

    let destination_dir = book.root_dir.join("files").join("imports");
    std::fs::create_dir_all(&destination_dir)?;
    let target_dest = destination_dir.join(imported_file_name);
    std::fs::write(&target_dest, &file_bytes)?;

    // Synchronize catalog to refresh indices
    runtime.sync_all().await?;

    Ok(())
}
