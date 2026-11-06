//! Book domain models, frontmatter validation, filesystem management, document import, and index document generation.

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::DomainError;
use crate::document::{
    TextExtractionMode, build_imported_book_markdown, decode_base64_document, inspect_document,
    read_local_document, render_generated_document_note, resolve_import_book_name, store_document,
    validate_document_file_name,
};
use crate::memory::{
    DEFAULT_MAX_PART_CHARS, IndexDocument, IndexMutation, IndexTarget, book_doc_id,
    book_doc_prefix, book_document_doc_id, content_hash_hex, split_text_into_parts,
};

const BOOK_FILE_NAME: &str = "BOOK.md";
const ALLOWED_BOOK_SUBDIRS: [&str; 6] = [
    "chapters",
    "notes",
    "references",
    "summaries",
    "assets",
    "files",
];
const ALLOWED_BOOK_ROOT_FILES: [&str; 2] = ["LICENSE.txt", "LICENSE"];

/// Complete representation of a Book directory on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookEntry {
    /// Validated book identifier (`name` in frontmatter and directory name).
    pub name: String,
    /// Validated book description from frontmatter.
    pub description: String,
    /// Path to the book directory on disk.
    pub root_dir: PathBuf,
    /// Full text of `BOOK.md`.
    pub book_markdown: String,
    /// All text files inside the book directory (`BOOK.md` first).
    pub files: Vec<BookFile>,
}

/// A single text file belonging to a [`BookEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookFile {
    /// Forward-slash relative path inside the book directory.
    pub relative_path: String,
    /// Full UTF-8 text content of the file.
    pub content: String,
}

/// Lightweight summary of a Book for `list_books`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSummary {
    /// Book identifier.
    pub name: String,
    /// Book description.
    pub description: String,
    /// Number of text files belonging to this book.
    pub file_count: usize,
    /// Relative paths of all text files in the book.
    pub files: Vec<String>,
}

/// Ranked book search match for `find_books`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookMatch {
    /// Book identifier.
    pub name: String,
    /// Book description.
    pub description: String,
    /// Relevance score.
    pub score: usize,
    /// Preview snippet.
    pub preview: String,
    /// Relative paths of all text files in the book.
    pub files: Vec<String>,
}

/// Input file specification when creating or updating a Book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookFileInput {
    /// Relative file path under allowed subdirectories.
    pub relative_path: String,
    /// UTF-8 file content.
    pub content: String,
}

/// Input parameters for creating a new Book on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBookInput {
    /// Optional book name; if omitted, parsed from `book_markdown` frontmatter.
    pub name: Option<String>,
    /// Optional description; used if `book_markdown` is omitted.
    pub description: Option<String>,
    /// Optional Markdown body; used when generating `BOOK.md` from `name` + `description`.
    pub body: Option<String>,
    /// Optional complete `BOOK.md` content.
    pub book_markdown: Option<String>,
    /// Additional files to write into the book directory.
    pub files: Vec<BookFileInput>,
}

/// Strategy for handling existing resource files when updating a Book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookFileUpdatePolicy {
    /// Keep existing resource files and overwrite or add any specified `files`.
    #[default]
    Merge,
    /// Remove all existing resource files and keep only `BOOK.md` plus specified `files`.
    ReplaceAll,
}

/// Input parameters for updating an existing Book on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateBookInput {
    /// Target book name to update.
    pub name: String,
    /// Optional updated description when regenerating `BOOK.md`.
    pub description: Option<String>,
    /// Optional updated Markdown body when regenerating `BOOK.md`.
    pub body: Option<String>,
    /// Optional complete replacement `BOOK.md` content.
    pub book_markdown: Option<String>,
    /// Resource files to write or overwrite.
    pub files: Vec<BookFileInput>,
    /// Relative paths of resource files to remove.
    pub remove_files: Vec<String>,
    /// Policy for existing resource files (`Merge` or `ReplaceAll`).
    pub file_policy: BookFileUpdatePolicy,
}

/// Report returned after creating, updating, or deleting a Book on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookMutationReport {
    /// Action label (`created`, `updated`, `deleted`).
    pub action: String,
    /// Book name.
    pub book_name: String,
    /// Book directory path on disk.
    pub root_dir: String,
    /// Number of text files remaining in the book directory.
    pub file_count: usize,
    /// Number of index chunks generated for this book.
    pub synced_book_parts: usize,
    /// Relative paths of text files in the book directory.
    pub files: Vec<String>,
    /// Index mutation to apply to the search store.
    pub index_mutation: IndexMutation,
}

/// Source of a document to import into a Book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportBookDocumentInput {
    /// Optional target book name; derived from the document filename when omitted.
    pub book_name: Option<String>,
    /// Optional description when creating a new book during import.
    pub description: Option<String>,
    /// Optional local filesystem path to read (must reside within `allowed_roots`).
    pub file_path: Option<String>,
    /// Optional explicit document filename (`*.pdf`, `*.docx`, `*.pptx`, `*.xlsx`, `*.xls`, `*.md`, `*.txt`).
    pub file_name: Option<String>,
    /// Optional Base64-encoded document bytes.
    pub content_base64: Option<String>,
    /// Optional additional tags to attach to imported document chunks.
    pub tags: Vec<String>,
    /// Whether to extract searchable text from the document (`true` by default).
    pub extract_text: bool,
}

/// Report returned after importing a document into a Book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookDocumentImportReport {
    /// Target book name.
    pub book_name: String,
    /// Whether a new book directory was created (`true`) or an existing one updated (`false`).
    pub created_book: bool,
    /// Book directory path on disk.
    pub root_dir: String,
    /// Normalized filename of the imported document.
    pub document_file_name: String,
    /// Relative path where the raw binary/file was archived (`files/imports/...`).
    pub stored_relative_path: String,
    /// Optional relative path of the generated Markdown note (`notes/imported-...`).
    pub generated_note_path: Option<String>,
    /// Detected document format label.
    pub document_format: String,
    /// Whether searchable text was extracted.
    pub extracted_text: bool,
    /// Number of extracted characters.
    pub extracted_chars: usize,
    /// Optional page/sheet/slide count.
    pub page_count: Option<usize>,
    /// Number of book chunks generated.
    pub synced_book_parts: usize,
    /// Number of direct document chunks generated.
    pub synced_document_chunks: usize,
    /// Non-fatal extraction warnings.
    pub warnings: Vec<String>,
    /// Index mutation to apply to the search store.
    pub index_mutation: IndexMutation,
}

/// Filesystem-backed catalog of Books.
#[derive(Debug, Clone)]
pub struct BookCatalog {
    root_dir: PathBuf,
}

impl BookCatalog {
    /// Create a [`BookCatalog`] rooted at `root_dir`.
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// Return the root directory path of the book catalog.
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Scan the catalog directory and return all valid [`BookEntry`] items sorted by name.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory reading fails or a book directory is invalid.
    pub fn collect_entries(&self) -> Result<Vec<BookEntry>, DomainError> {
        if !self.root_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for dir_entry in fs::read_dir(&self.root_dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if dir_name.starts_with('.') {
                continue;
            }
            let book_md_path = path.join(BOOK_FILE_NAME);
            if !book_md_path.is_file() {
                continue;
            }
            entries.push(load_book_entry(&path)?);
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// List lightweight summaries for all valid books in the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if scanning fails.
    pub fn list_summaries(&self) -> Result<Vec<BookSummary>, DomainError> {
        let entries = self.collect_entries()?;
        Ok(entries
            .into_iter()
            .map(|entry| BookSummary {
                name: entry.name,
                description: entry.description,
                file_count: entry.files.len(),
                files: entry.files.into_iter().map(|f| f.relative_path).collect(),
            })
            .collect())
    }

    /// Search books by lexical query across name, description, file paths, and content.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `query` is empty or catalog scanning fails.
    pub fn find_matches(&self, query: &str, limit: usize) -> Result<Vec<BookMatch>, DomainError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("query 不能为空".to_string()));
        }

        let entries = self.collect_entries()?;
        let mut matches: Vec<BookMatch> = entries
            .into_iter()
            .filter_map(|entry| {
                let score = score_book_entry(&entry, trimmed);
                if score == 0 {
                    return None;
                }
                let preview = build_book_preview(&entry, trimmed);
                let files = entry.files.into_iter().map(|f| f.relative_path).collect();
                Some(BookMatch {
                    name: entry.name,
                    description: entry.description,
                    score,
                    preview,
                    files,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        matches.truncate(limit.clamp(1, 50));
        Ok(matches)
    }

    /// Load the full `BOOK.md` content and resource list for `book_name`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `book_name` is invalid or does not exist.
    pub fn load_book(&self, book_name: &str) -> Result<BookEntry, DomainError> {
        let normalized = book_name.trim();
        validate_book_name(normalized)?;
        let book_dir = self.root_dir.join(normalized);
        if !book_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "未找到名为 `{normalized}` 的 book"
            )));
        }
        load_book_entry(&book_dir)
    }

    /// Load a specific relative file (`BOOK.md` or resource file) from `book_name`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `book_name` or `relative_path` is invalid or missing.
    pub fn load_resource(
        &self,
        book_name: &str,
        relative_path: &str,
    ) -> Result<BookFile, DomainError> {
        let entry = self.load_book(book_name)?;
        let clean_path = if relative_path.trim().is_empty()
            || relative_path.trim().eq_ignore_ascii_case(BOOK_FILE_NAME)
        {
            BOOK_FILE_NAME.to_string()
        } else {
            normalize_relative_book_path(relative_path)?
        };

        entry
            .files
            .into_iter()
            .find(|f| f.relative_path == clean_path)
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "在 book `{}` 中未找到文件 `{clean_path}`",
                    entry.name
                ))
            })
    }

    /// Create a new Book on disk and build its corresponding [`IndexMutation`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if validation fails or the book directory already exists.
    pub fn create_book(&self, input: &CreateBookInput) -> Result<BookMutationReport, DomainError> {
        let book_markdown = resolve_book_markdown(
            input.name.as_deref(),
            input.description.as_deref(),
            input.body.as_deref(),
            input.book_markdown.as_deref(),
        )?;
        let (parsed_name, _) = parse_book_frontmatter(&book_markdown)?;
        if let Some(explicit_name) = input
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            validate_book_name(explicit_name)?;
            if explicit_name != parsed_name {
                return Err(DomainError::Validation(format!(
                    "参数 name `{explicit_name}` 与 BOOK.md frontmatter 中的 name `{parsed_name}` 不一致"
                )));
            }
        }
        validate_book_document(&parsed_name, &book_markdown)?;

        let normalized_files = normalize_book_file_inputs(&input.files)?;
        fs::create_dir_all(&self.root_dir)?;
        let book_dir = self.root_dir.join(&parsed_name);
        if book_dir.exists() {
            return Err(DomainError::AlreadyExists(format!(
                "book `{parsed_name}` 已存在"
            )));
        }

        fs::create_dir_all(&book_dir)?;
        if let Err(err) = write_book_files(&book_dir, &book_markdown, &normalized_files) {
            let _ = fs::remove_dir_all(&book_dir);
            return Err(err);
        }

        let entry = load_book_entry(&book_dir)?;
        let upserts = build_book_index_documents(&entry);
        let synced_parts = upserts.len();
        let files: Vec<String> = entry.files.into_iter().map(|f| f.relative_path).collect();

        Ok(BookMutationReport {
            action: "created".to_string(),
            book_name: entry.name.clone(),
            root_dir: book_dir.display().to_string(),
            file_count: files.len(),
            synced_book_parts: synced_parts,
            files,
            index_mutation: IndexMutation {
                target: IndexTarget::Books,
                owner_prefix: Some(book_doc_prefix(&entry.name)),
                upserts,
                delete_doc_ids: Vec::new(),
            },
        })
    }

    /// Update an existing Book on disk and build its corresponding [`IndexMutation`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the book does not exist or validation fails.
    pub fn update_book(&self, input: &UpdateBookInput) -> Result<BookMutationReport, DomainError> {
        let book_name = input.name.trim();
        validate_book_name(book_name)?;
        let book_dir = self.root_dir.join(book_name);
        if !book_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "book `{book_name}` 不存在，无法更新"
            )));
        }

        let current_entry = load_book_entry(&book_dir)?;
        let next_markdown =
            if input.book_markdown.is_some() || input.description.is_some() || input.body.is_some()
            {
                let existing_body = extract_markdown_body(&current_entry.book_markdown);
                resolve_book_markdown(
                    Some(book_name),
                    input
                        .description
                        .as_deref()
                        .or(Some(current_entry.description.as_str())),
                    input.body.as_deref().or(Some(existing_body)),
                    input.book_markdown.as_deref(),
                )?
            } else {
                current_entry.book_markdown.clone()
            };

        validate_book_document(book_name, &next_markdown)?;
        let normalized_files = normalize_book_file_inputs(&input.files)?;
        let remove_paths: Vec<String> = input
            .remove_files
            .iter()
            .map(|p| normalize_relative_book_path(p))
            .collect::<Result<Vec<_>, _>>()?;

        if input.file_policy == BookFileUpdatePolicy::ReplaceAll {
            for existing in &current_entry.files {
                if existing.relative_path != BOOK_FILE_NAME {
                    let target = book_dir.join(&existing.relative_path);
                    if target.is_file() {
                        fs::remove_file(target)?;
                    }
                }
            }
        } else {
            for rel in &remove_paths {
                let target = book_dir.join(rel);
                if target.is_file() {
                    fs::remove_file(target)?;
                }
            }
        }

        write_book_files(&book_dir, &next_markdown, &normalized_files)?;
        let updated_entry = load_book_entry(&book_dir)?;
        let upserts = build_book_index_documents(&updated_entry);
        let synced_parts = upserts.len();
        let files: Vec<String> = updated_entry
            .files
            .into_iter()
            .map(|f| f.relative_path)
            .collect();

        Ok(BookMutationReport {
            action: "updated".to_string(),
            book_name: updated_entry.name.clone(),
            root_dir: book_dir.display().to_string(),
            file_count: files.len(),
            synced_book_parts: synced_parts,
            files,
            index_mutation: IndexMutation {
                target: IndexTarget::Books,
                owner_prefix: Some(book_doc_prefix(&updated_entry.name)),
                upserts,
                delete_doc_ids: Vec::new(),
            },
        })
    }

    /// Delete a Book directory from disk and return the [`IndexMutation`] to purge its chunks.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `book_name` is invalid or does not exist.
    pub fn delete_book(&self, book_name: &str) -> Result<BookMutationReport, DomainError> {
        let normalized = book_name.trim();
        validate_book_name(normalized)?;
        let book_dir = self.root_dir.join(normalized);
        if !book_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "book `{normalized}` 不存在，无法删除"
            )));
        }

        let entry = load_book_entry(&book_dir)?;
        let old_docs = build_book_index_documents(&entry);
        let delete_doc_ids: Vec<String> = old_docs.into_iter().map(|d| d.doc_id).collect();
        let removed_files: Vec<String> = entry.files.into_iter().map(|f| f.relative_path).collect();

        fs::remove_dir_all(&book_dir)?;

        Ok(BookMutationReport {
            action: "deleted".to_string(),
            book_name: normalized.to_string(),
            root_dir: book_dir.display().to_string(),
            file_count: 0,
            synced_book_parts: 0,
            files: removed_files,
            index_mutation: IndexMutation {
                target: IndexTarget::Books,
                owner_prefix: Some(book_doc_prefix(normalized)),
                upserts: Vec::new(),
                delete_doc_ids,
            },
        })
    }

    /// Import a local or Base64 document into a Book, archiving the raw document,
    /// extracting text into `notes/`, and producing index chunks.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if source arguments conflict, path security checks fail,
    /// or document storage fails.
    pub fn import_document(
        &self,
        input: &ImportBookDocumentInput,
        allowed_roots: &[PathBuf],
        max_file_bytes: usize,
    ) -> Result<BookDocumentImportReport, DomainError> {
        let (document_file_name, bytes, source_label) =
            resolve_imported_document_source(input, allowed_roots, max_file_bytes)?;

        let book_name = resolve_import_book_name(input.book_name.as_deref(), &document_file_name)?;
        let extraction_mode = if input.extract_text {
            TextExtractionMode::Extract
        } else {
            TextExtractionMode::MetadataOnly
        };
        let inspection = inspect_document(&bytes, &document_file_name, extraction_mode)?;

        fs::create_dir_all(&self.root_dir)?;
        let book_dir = self.root_dir.join(&book_name);
        let created_book = !book_dir.exists();
        fs::create_dir_all(&book_dir)?;

        let (stored_relative_path, _stored_abs_path) =
            store_document(&book_dir, &document_file_name, &bytes)?;

        let mut generated_note_path: Option<String> = None;
        let mut extracted_chars = 0usize;

        if let Some(extracted_text) = &inspection.extracted_text {
            extracted_chars = extracted_text.chars().count();
            let stem = Path::new(&document_file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");
            let note_rel = format!("notes/imported-{stem}.md");
            let note_abs = book_dir.join(&note_rel);
            if let Some(parent) = note_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            let note_md = render_generated_document_note(
                &document_file_name,
                &stored_relative_path,
                &source_label,
                &inspection,
                extracted_text,
            );
            fs::write(&note_abs, note_md)?;
            generated_note_path = Some(note_rel);
        }

        let book_md_path = book_dir.join(BOOK_FILE_NAME);
        if !book_md_path.is_file() {
            let generated_book_md = build_imported_book_markdown(
                &book_name,
                input.description.as_deref(),
                &document_file_name,
                &stored_relative_path,
                generated_note_path.as_deref(),
                &inspection,
            );
            fs::write(&book_md_path, generated_book_md)?;
        }

        let entry = load_book_entry(&book_dir)?;
        let mut upserts = build_book_index_documents(&entry);
        let synced_book_parts = upserts.len();

        let mut synced_document_chunks = 0usize;
        if let Some(extracted_text) = &inspection.extracted_text {
            let doc_hash = content_hash_hex(&bytes);
            let doc_chunks = build_imported_document_chunks(
                &entry,
                &document_file_name,
                &stored_relative_path,
                &inspection.format_label,
                &doc_hash,
                extracted_text,
                &input.tags,
            );
            synced_document_chunks = doc_chunks.len();
            upserts.extend(doc_chunks);
        }

        Ok(BookDocumentImportReport {
            book_name: entry.name.clone(),
            created_book,
            root_dir: book_dir.display().to_string(),
            document_file_name,
            stored_relative_path,
            generated_note_path,
            document_format: inspection.format_label,
            extracted_text: inspection.extracted_text.is_some(),
            extracted_chars,
            page_count: inspection.page_count,
            synced_book_parts,
            synced_document_chunks,
            warnings: inspection.warnings,
            index_mutation: IndexMutation {
                target: IndexTarget::Books,
                owner_prefix: Some(book_doc_prefix(&entry.name)),
                upserts,
                delete_doc_ids: Vec::new(),
            },
        })
    }

    /// Build all [`IndexDocument`] chunks for every valid Book currently on disk.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if scanning fails.
    pub fn build_all_index_documents(&self) -> Result<(usize, Vec<IndexDocument>), DomainError> {
        let entries = self.collect_entries()?;
        let count = entries.len();
        let mut docs = Vec::new();
        for entry in &entries {
            docs.extend(build_book_index_documents(entry));
        }
        Ok((count, docs))
    }
}

/// Build deterministic [`IndexDocument`] chunks for a single [`BookEntry`].
#[must_use]
pub fn build_book_index_documents(entry: &BookEntry) -> Vec<IndexDocument> {
    let now = Utc::now().to_rfc3339();
    let mut docs = Vec::new();

    for file in &entry.files {
        let parts = split_text_into_parts(&file.content, DEFAULT_MAX_PART_CHARS);
        let part_total = parts.len();
        for (idx, part_text) in parts.into_iter().enumerate() {
            let part_index = idx + 1;
            let doc_id = book_doc_id(&entry.name, &file.relative_path, part_index);
            let uri = if part_total == 1 {
                format!("book://{}/{}", entry.name, file.relative_path)
            } else {
                format!(
                    "book://{}/{}#part-{part_index}",
                    entry.name, file.relative_path
                )
            };
            let title = if file.relative_path == BOOK_FILE_NAME {
                format!("Book {}", entry.name)
            } else {
                format!("Book {} - {}", entry.name, file.relative_path)
            };
            let hash_input = format!(
                "{}\n{}\n{}\n{}",
                entry.name, entry.description, file.relative_path, part_text
            );
            let content_hash = content_hash_hex(hash_input.as_bytes());
            let tags = vec![
                "kind:book".to_string(),
                format!("book:{}", entry.name),
                format!("path:{}", file.relative_path),
            ];

            let mut extra = Map::new();
            extra.insert("book_name".to_string(), Value::String(entry.name.clone()));
            extra.insert(
                "book_description".to_string(),
                Value::String(entry.description.clone()),
            );

            docs.push(IndexDocument {
                doc_id,
                target: IndexTarget::Books,
                title,
                uri,
                text: part_text,
                tags,
                source_kind: "book".to_string(),
                owner_name: entry.name.clone(),
                relative_path: file.relative_path.clone(),
                part_index,
                part_total,
                content_hash,
                updated_at: now.clone(),
                extra,
            });
        }
    }

    docs
}

fn build_imported_document_chunks(
    entry: &BookEntry,
    document_file_name: &str,
    stored_relative_path: &str,
    document_format: &str,
    document_hash: &str,
    extracted_text: &str,
    user_tags: &[String],
) -> Vec<IndexDocument> {
    let now = Utc::now().to_rfc3339();
    let parts = split_text_into_parts(extracted_text, DEFAULT_MAX_PART_CHARS);
    let part_total = parts.len();
    let mut docs = Vec::with_capacity(part_total);

    for (idx, part_text) in parts.into_iter().enumerate() {
        let part_index = idx + 1;
        let doc_id = book_document_doc_id(&entry.name, document_hash, part_index);
        let uri = if part_total == 1 {
            format!("book://{}/{}", entry.name, stored_relative_path)
        } else {
            format!(
                "book://{}/{}#part-{part_index}",
                entry.name, stored_relative_path
            )
        };
        let title = format!("Book {} - {}", entry.name, document_file_name);
        let hash_input = format!(
            "{}\n{}\n{}\n{}",
            entry.name, document_hash, part_index, part_text
        );
        let content_hash = content_hash_hex(hash_input.as_bytes());

        let mut tags = vec![
            "kind:book_document".to_string(),
            format!("book:{}", entry.name),
            format!("path:{stored_relative_path}"),
            format!("format:{document_format}"),
        ];
        for tag in user_tags {
            let trimmed = tag.trim();
            if !trimmed.is_empty() && !tags.iter().any(|existing| existing == trimmed) {
                tags.push(trimmed.to_string());
            }
        }

        let mut extra = Map::new();
        extra.insert("book_name".to_string(), Value::String(entry.name.clone()));
        extra.insert(
            "book_description".to_string(),
            Value::String(entry.description.clone()),
        );
        extra.insert(
            "document_file_name".to_string(),
            Value::String(document_file_name.to_string()),
        );
        extra.insert(
            "document_format".to_string(),
            Value::String(document_format.to_string()),
        );

        docs.push(IndexDocument {
            doc_id,
            target: IndexTarget::Books,
            title,
            uri,
            text: part_text,
            tags,
            source_kind: "book_document".to_string(),
            owner_name: entry.name.clone(),
            relative_path: stored_relative_path.to_string(),
            part_index,
            part_total,
            content_hash,
            updated_at: now.clone(),
            extra,
        });
    }

    docs
}

/// Validate that `name` is 1..=64 ASCII lowercase alphanumeric characters or single hyphens.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `name` violates the naming rules.
pub fn validate_book_name(name: &str) -> Result<(), DomainError> {
    if name.is_empty() || name.len() > 64 {
        return Err(DomainError::Validation(format!(
            "book 名称 `{name}` 长度必须在 1..=64 之间"
        )));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(DomainError::Validation(format!(
            "book 名称 `{name}` 不能以连字符开头/结尾或包含连续连字符"
        )));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(DomainError::Validation(format!(
            "book 名称 `{name}` 仅允许使用小写字母、数字和单连字符"
        )));
    }
    Ok(())
}

/// Parse `(name, description)` from the YAML frontmatter of `BOOK.md`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if frontmatter is missing or malformed.
pub fn parse_book_frontmatter(markdown: &str) -> Result<(String, String), DomainError> {
    let normalized = markdown.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(DomainError::Validation(
            "BOOK.md 必须以 YAML frontmatter (`---`) 开头".to_string(),
        ));
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---\n") else {
        return Err(DomainError::Validation(
            "BOOK.md YAML frontmatter 缺少结束分隔符 `---`".to_string(),
        ));
    };

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;

    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(DomainError::Validation(format!(
                "BOOK.md frontmatter 行格式非法: `{line}`"
            )));
        };
        let key = key.trim();
        let value = trim_yaml_scalar(value.trim());
        match key {
            "name" => name = Some(value),
            "description" => description = Some(value),
            other => {
                return Err(DomainError::Validation(format!(
                    "BOOK.md frontmatter 仅允许 `name` 和 `description` 字段，发现未知字段 `{other}`"
                )));
            }
        }
    }

    let name = name
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DomainError::Validation("BOOK.md frontmatter 缺少 `name`".to_string()))?;
    let description = description.filter(|s| !s.is_empty()).ok_or_else(|| {
        DomainError::Validation("BOOK.md frontmatter 缺少 `description`".to_string())
    })?;

    Ok((name, description))
}

/// Validate `BOOK.md` frontmatter and body constraints against `expected_name`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the frontmatter or body is invalid.
pub fn validate_book_document(expected_name: &str, markdown: &str) -> Result<String, DomainError> {
    validate_book_name(expected_name)?;
    let (name, description) = parse_book_frontmatter(markdown)?;
    validate_book_name(&name)?;
    if name != expected_name {
        return Err(DomainError::Validation(format!(
            "目录名 `{expected_name}` 与 BOOK.md frontmatter 中的 name `{name}` 不一致"
        )));
    }
    if description.chars().count() > 1_024 {
        return Err(DomainError::Validation(
            "BOOK.md description 长度不能超过 1024 个字符".to_string(),
        ));
    }
    if description.contains('<') || description.contains('>') {
        return Err(DomainError::Validation(
            "BOOK.md description 不能包含尖括号 `<` 或 `>`".to_string(),
        ));
    }
    let body = extract_markdown_body(markdown).trim();
    if body.is_empty() {
        return Err(DomainError::Validation(
            "BOOK.md 正文内容不能为空".to_string(),
        ));
    }
    Ok(description)
}

/// Validate and normalize a relative resource file path inside a Book directory.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the path escapes the book directory or is not in an allowed location.
pub fn normalize_relative_book_path(raw_path: &str) -> Result<String, DomainError> {
    let trimmed = raw_path.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err(DomainError::Validation(
            "book 资源文件路径不能为空".to_string(),
        ));
    }
    let path = Path::new(&trimmed);
    let mut parts = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.starts_with('.') {
                    return Err(DomainError::Validation(format!(
                        "book 资源路径 `{trimmed}` 不能包含隐藏文件或目录"
                    )));
                }
                parts.push(s.into_owned());
            }
            _ => {
                return Err(DomainError::Validation(format!(
                    "book 资源路径 `{trimmed}` 必须是安全的相对路径"
                )));
            }
        }
    }

    if parts.is_empty() {
        return Err(DomainError::Validation(
            "book 资源文件路径不能为空".to_string(),
        ));
    }

    if parts.len() == 1 {
        let single = &parts[0];
        if ALLOWED_BOOK_ROOT_FILES.contains(&single.as_str()) {
            return Ok(single.clone());
        }
        return Err(DomainError::Validation(format!(
            "book 根目录仅允许 {BOOK_FILE_NAME} 或 LICENSE.txt，不允许 `{single}`"
        )));
    }

    let top = &parts[0];
    if !ALLOWED_BOOK_SUBDIRS.contains(&top.as_str()) {
        return Err(DomainError::Validation(format!(
            "book 资源文件必须位于 chapters/、notes/、references/、summaries/、assets/ 或 files/ 下，非法路径 `{trimmed}`"
        )));
    }

    Ok(parts.join("/"))
}

/// Compute a lexical relevance score for `entry` against `query`.
#[must_use]
pub fn score_book_entry(entry: &BookEntry, query: &str) -> usize {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }

    let name_lower = entry.name.to_lowercase();
    let desc_lower = entry.description.to_lowercase();
    let body_lower = entry.book_markdown.to_lowercase();

    let mut score = 0usize;
    if name_lower == q {
        score += 120;
    } else if name_lower.contains(&q) {
        score += 70;
    }
    if desc_lower.contains(&q) {
        score += 45;
    }
    if body_lower.contains(&q) {
        score += 25;
    }

    for token in q.split_whitespace().filter(|t| t.len() >= 2) {
        if name_lower.contains(token) {
            score += 25;
        }
        if desc_lower.contains(token) {
            score += 15;
        }
        if body_lower.contains(token) {
            score += 8;
        }
        for file in &entry.files {
            if file.relative_path.to_lowercase().contains(token) {
                score += 6;
            }
            if file.content.to_lowercase().contains(token) {
                score += 4;
            }
        }
    }

    score
}

fn build_book_preview(entry: &BookEntry, query: &str) -> String {
    let q_lower = query.trim().to_lowercase();
    for file in &entry.files {
        for line in file.content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.to_lowercase().contains(&q_lower) {
                return trimmed.chars().take(180).collect();
            }
        }
    }
    entry.description.chars().take(180).collect()
}

fn load_book_entry(book_dir: &Path) -> Result<BookEntry, DomainError> {
    let dir_name = book_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            DomainError::Validation(format!("无法识别 book 目录名称: {}", book_dir.display()))
        })?;

    let book_md_path = book_dir.join(BOOK_FILE_NAME);
    let book_markdown = fs::read_to_string(&book_md_path)?;
    let description = validate_book_document(dir_name, &book_markdown)?;

    let mut resource_files = Vec::new();
    collect_book_files_recursive(book_dir, book_dir, &mut resource_files)?;
    resource_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut files = Vec::with_capacity(1 + resource_files.len());
    files.push(BookFile {
        relative_path: BOOK_FILE_NAME.to_string(),
        content: book_markdown.clone(),
    });
    files.extend(resource_files);

    Ok(BookEntry {
        name: dir_name.to_string(),
        description,
        root_dir: book_dir.to_path_buf(),
        book_markdown,
        files,
    })
}

fn collect_book_files_recursive(
    root: &Path,
    current: &Path,
    out: &mut Vec<BookFile>,
) -> Result<(), DomainError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if current == root && !ALLOWED_BOOK_SUBDIRS.contains(&name) {
                continue;
            }
            collect_book_files_recursive(root, &path, out)?;
        } else if path.is_file() {
            if current == root && name == BOOK_FILE_NAME {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with("files/imports/") {
                    // Binary/raw imported files are archived here; their extracted text lives in notes/
                    continue;
                }
                if normalize_relative_book_path(&rel_str).is_ok()
                    && let Ok(content) = fs::read_to_string(&path)
                    && !content.trim().is_empty()
                {
                    out.push(BookFile {
                        relative_path: rel_str,
                        content,
                    });
                }
            }
        }
    }
    Ok(())
}

fn resolve_book_markdown(
    name: Option<&str>,
    description: Option<&str>,
    body: Option<&str>,
    book_markdown: Option<&str>,
) -> Result<String, DomainError> {
    if let Some(raw_md) = book_markdown.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(format!("{raw_md}\n"));
    }

    let name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            DomainError::Validation(
                "未提供 book_markdown 时，必须同时提供 name 和 description".to_string(),
            )
        })?;
    let description = description
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            DomainError::Validation(
                "未提供 book_markdown 时，必须同时提供 name 和 description".to_string(),
            )
        })?;
    validate_book_name(name)?;

    let body_content = body
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("# {name}\n\n{description}"), str::to_string);

    let mut rendered = String::new();
    let _ = writeln!(rendered, "---");
    let _ = writeln!(rendered, "name: {name}");
    let _ = writeln!(rendered, "description: {description}");
    let _ = writeln!(rendered, "---");
    let _ = writeln!(rendered);
    let _ = writeln!(rendered, "{body_content}");
    Ok(rendered)
}

fn normalize_book_file_inputs(files: &[BookFileInput]) -> Result<Vec<BookFileInput>, DomainError> {
    let mut normalized = Vec::with_capacity(files.len());
    for file in files {
        let clean_path = normalize_relative_book_path(&file.relative_path)?;
        if file.content.trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "文件 `{clean_path}` 的内容不能为空"
            )));
        }
        normalized.push(BookFileInput {
            relative_path: clean_path,
            content: file.content.clone(),
        });
    }
    Ok(normalized)
}

fn write_book_files(
    book_dir: &Path,
    book_markdown: &str,
    files: &[BookFileInput],
) -> Result<(), DomainError> {
    fs::write(book_dir.join(BOOK_FILE_NAME), book_markdown)?;
    for file in files {
        let target = book_dir.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, &file.content)?;
    }
    Ok(())
}

fn extract_markdown_body(markdown: &str) -> &str {
    let normalized = markdown.trim_start();
    if let Some(rest) = normalized.strip_prefix("---\n")
        && let Some((_fm, body)) = rest.split_once("\n---\n")
    {
        return body;
    }
    if let Some(rest) = normalized.strip_prefix("---\r\n")
        && let Some((_fm, body)) = rest.split_once("\r\n---\r\n")
    {
        return body;
    }
    markdown
}

fn trim_yaml_scalar(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolve_imported_document_source(
    input: &ImportBookDocumentInput,
    allowed_roots: &[PathBuf],
    max_file_bytes: usize,
) -> Result<(String, Vec<u8>, String), DomainError> {
    let has_path = input
        .file_path
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    let has_b64 = input
        .content_base64
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());

    if has_path == has_b64 {
        return Err(DomainError::Validation(
            "import_book_document 必须在 file_path 与 content_base64 之间二选一".to_string(),
        ));
    }

    if let Some(path) = input
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        read_local_document(
            path,
            input.file_name.as_deref(),
            allowed_roots,
            max_file_bytes,
        )
    } else {
        let raw_name = input.file_name.as_deref().ok_or_else(|| {
            DomainError::Validation("使用 content_base64 导入时必须提供 file_name".to_string())
        })?;
        let clean_name = validate_document_file_name(raw_name)?;
        let b64 = input.content_base64.as_deref().unwrap_or_default();
        let decoded = decode_base64_document(b64, max_file_bytes, &clean_name)?;
        Ok((clean_name.clone(), decoded, format!("base64:{clean_name}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use tempfile::TempDir;

    #[test]
    fn book_lifecycle_and_document_import_work() {
        let dir = TempDir::new().expect("tempdir");
        let catalog = BookCatalog::new(dir.path());

        let report = catalog
            .import_document(
                &ImportBookDocumentInput {
                    book_name: Some("architecture-guide".to_string()),
                    description: Some("Architecture transformation guide".to_string()),
                    file_path: None,
                    file_name: Some("overview.md".to_string()),
                    content_base64: Some(
                        BASE64_STANDARD.encode("# Overview\n\nXERJ is the unified search engine."),
                    ),
                    tags: vec!["topic:xerj".to_string()],
                    extract_text: true,
                },
                &[],
                1024 * 1024,
            )
            .expect("import document");

        assert!(report.created_book);
        assert!(report.extracted_text);
        assert!(report.synced_book_parts >= 2);
        assert_eq!(report.synced_document_chunks, 1);

        let loaded = catalog
            .load_book("architecture-guide")
            .expect("load imported book");
        assert_eq!(loaded.name, "architecture-guide");
    }
}
