//! Document validation, safe filesystem loading, and pure-Rust text extraction for Books.

use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use calamine::{Data, Reader, Sheets, open_workbook_auto_from_rs};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::DomainError;
use crate::book::validate_book_name;

/// Inspection and text extraction result for an imported Book document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookDocumentInspection {
    /// Human-readable format label (`pdf`, `docx`, `pptx`, `xlsx`, `xls`, `markdown`, `text`).
    pub format_label: String,
    /// Extracted searchable text body, if any.
    pub extracted_text: Option<String>,
    /// Optional page or slide/sheet count discovered during extraction.
    pub page_count: Option<usize>,
    /// Non-fatal extraction warnings.
    pub warnings: Vec<String>,
}

/// Validate that `file_name` is a single safe file name with a supported document extension.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `file_name` is empty, contains path separators,
/// or uses an unsupported extension.
pub fn validate_document_file_name(file_name: &str) -> Result<String, DomainError> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation("file_name 不能为空".to_string()));
    }

    let path = Path::new(trimmed);
    let mut components = path.components();
    let Some(Component::Normal(component)) = components.next() else {
        return Err(DomainError::Validation(format!(
            "file_name `{trimmed}` 必须是单个文件名，不能包含目录路径"
        )));
    };
    if components.next().is_some() || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(DomainError::Validation(format!(
            "file_name `{trimmed}` 必须是单个文件名，不能包含目录分隔符"
        )));
    }

    let normalized = component.to_string_lossy().to_string();
    let ext = Path::new(&normalized)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| DomainError::Validation(format!("文档 `{normalized}` 缺少文件扩展名")))?;

    if !matches!(
        ext.as_str(),
        "pdf" | "docx" | "pptx" | "xlsx" | "xls" | "md" | "txt"
    ) {
        return Err(DomainError::Validation(format!(
            "不支持的文档扩展名 `.{ext}`，当前支持 pdf/docx/pptx/xlsx/xls/md/txt"
        )));
    }

    Ok(normalized)
}

/// Validate that a document's byte length is non-empty and within `max_file_bytes`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `size == 0` or `size > max_file_bytes`.
pub fn validate_document_size(
    size: u64,
    max_file_bytes: usize,
    source_label: &str,
) -> Result<(), DomainError> {
    if size == 0 {
        return Err(DomainError::Validation(format!(
            "文档 `{source_label}` 内容为空"
        )));
    }
    if size > max_file_bytes as u64 {
        return Err(DomainError::Validation(format!(
            "文档 `{source_label}` 大小 {size} 字节超过上限 {max_file_bytes} 字节"
        )));
    }
    Ok(())
}

/// Decode a Base64-encoded document payload and enforce `max_file_bytes`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `content_base64` is empty, invalid Base64,
/// or exceeds `max_file_bytes`.
pub fn decode_base64_document(
    content_base64: &str,
    max_file_bytes: usize,
    source_label: &str,
) -> Result<Vec<u8>, DomainError> {
    let compact: String = content_base64
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    if compact.is_empty() {
        return Err(DomainError::Validation(
            "content_base64 不能为空".to_string(),
        ));
    }

    let bytes = BASE64_STANDARD.decode(compact).map_err(|err| {
        DomainError::Validation(format!("解析 `{source_label}` 的 Base64 内容失败: {err}"))
    })?;
    validate_document_size(bytes.len() as u64, max_file_bytes, source_label)?;
    Ok(bytes)
}

/// Read a local document file after verifying it resides under `allowed_roots` and respects `max_file_bytes`.
///
/// # Errors
///
/// Returns [`DomainError`] if `allowed_roots` is empty, the path is outside all allowed roots,
/// the file is missing, or its size violates `max_file_bytes`.
pub fn read_local_document(
    raw_path: &str,
    explicit_file_name: Option<&str>,
    allowed_roots: &[PathBuf],
    max_file_bytes: usize,
) -> Result<(String, Vec<u8>, String), DomainError> {
    if allowed_roots.is_empty() {
        return Err(DomainError::Validation(
            "当前未配置允许导入本地文档的目录白名单 (document_import.allowed_roots)".to_string(),
        ));
    }

    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation("file_path 不能为空".to_string()));
    }

    let canonical_file = fs::canonicalize(trimmed)
        .map_err(|err| DomainError::Validation(format!("无法访问本地文档 `{trimmed}`: {err}")))?;

    let mut allowed = false;
    for root in allowed_roots {
        if let Ok(canonical_root) = fs::canonicalize(root)
            && canonical_file.starts_with(&canonical_root)
        {
            allowed = true;
            break;
        }
    }

    if !allowed {
        return Err(DomainError::Validation(format!(
            "本地文档 `{}` 不在允许导入的目录白名单内",
            canonical_file.display()
        )));
    }

    let metadata = fs::metadata(&canonical_file)?;
    if !metadata.is_file() {
        return Err(DomainError::Validation(format!(
            "`{}` 不是普通文件",
            canonical_file.display()
        )));
    }

    let source_display = canonical_file.display().to_string();
    validate_document_size(metadata.len(), max_file_bytes, &source_display)?;

    let derived_name =
        if let Some(name) = explicit_file_name.map(str::trim).filter(|s| !s.is_empty()) {
            validate_document_file_name(name)?
        } else {
            let os_name = canonical_file
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    DomainError::Validation(format!("无法从 `{source_display}` 推导文件名"))
                })?;
            validate_document_file_name(os_name)?
        };

    let bytes = fs::read(&canonical_file)?;
    validate_document_size(bytes.len() as u64, max_file_bytes, &source_display)?;

    Ok((derived_name, bytes, source_display))
}

/// Resolve or derive a valid `book_name` from an optional explicit name and a document filename.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the explicit name is invalid or a valid slug cannot be derived.
pub fn resolve_import_book_name(
    book_name: Option<&str>,
    file_name: &str,
) -> Result<String, DomainError> {
    if let Some(explicit) = book_name.map(str::trim).filter(|s| !s.is_empty()) {
        validate_book_name(explicit)?;
        return Ok(explicit.to_string());
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    let candidate = if slug.is_empty() {
        "imported-book".to_string()
    } else if slug.len() > 64 {
        slug[..64].trim_end_matches('-').to_string()
    } else {
        slug
    };
    validate_book_name(&candidate)?;
    Ok(candidate)
}

/// Store raw document bytes under `<book_dir>/files/imports/<file_name>`, choosing a suffixed
/// name if a different file with the same name already exists.
///
/// # Errors
///
/// Returns [`DomainError`] if directory creation or file writing fails.
pub fn store_document(
    book_dir: &Path,
    file_name: &str,
    bytes: &[u8],
) -> Result<(String, PathBuf), DomainError> {
    let imports_dir = book_dir.join("files").join("imports");
    fs::create_dir_all(&imports_dir)?;

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let mut chosen_name = file_name.to_string();
    let mut counter = 2usize;
    loop {
        let candidate_path = imports_dir.join(&chosen_name);
        if !candidate_path.exists() {
            fs::write(&candidate_path, bytes)?;
            let relative = format!("files/imports/{chosen_name}");
            return Ok((relative, candidate_path));
        }
        if let Ok(existing) = fs::read(&candidate_path)
            && existing == bytes
        {
            let relative = format!("files/imports/{chosen_name}");
            return Ok((relative, candidate_path));
        }
        chosen_name = if ext.is_empty() {
            format!("{stem}-{counter}")
        } else {
            format!("{stem}-{counter}.{ext}")
        };
        counter += 1;
    }
}

/// Controls whether [`inspect_document`] extracts full text or inspects metadata only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextExtractionMode {
    /// Parse and extract searchable plain text from the document.
    #[default]
    Extract,
    /// Inspect file format metadata only without extracting text.
    MetadataOnly,
}

/// Inspect a document and optionally extract its plain text using pure-Rust parsers.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the document format cannot be inspected.
pub fn inspect_document(
    bytes: &[u8],
    file_name: &str,
    mode: TextExtractionMode,
) -> Result<BookDocumentInspection, DomainError> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let format_label = match ext.as_str() {
        "md" => "markdown".to_string(),
        "txt" => "text".to_string(),
        other => other.to_string(),
    };

    if mode == TextExtractionMode::MetadataOnly {
        return Ok(BookDocumentInspection {
            format_label,
            extracted_text: None,
            page_count: None,
            warnings: Vec::new(),
        });
    }

    let mut warnings = Vec::new();
    let (extracted, page_count) = match ext.as_str() {
        "md" | "txt" => (Some(String::from_utf8_lossy(bytes).into_owned()), None),
        "pdf" => match extract_pdf_text(bytes) {
            Ok((text, pages)) => (Some(text), Some(pages)),
            Err(err) => {
                warnings.push(format!("PDF 文本提取告警: {err}"));
                (None, None)
            }
        },
        "docx" => match extract_docx_text(bytes) {
            Ok(text) => (Some(text), None),
            Err(err) => {
                warnings.push(format!("DOCX 文本提取告警: {err}"));
                (None, None)
            }
        },
        "pptx" => match extract_pptx_text(bytes) {
            Ok((text, slides)) => (Some(text), Some(slides)),
            Err(err) => {
                warnings.push(format!("PPTX 文本提取告警: {err}"));
                (None, None)
            }
        },
        "xlsx" | "xls" => match extract_spreadsheet_text(bytes) {
            Ok((text, sheets)) => (Some(text), Some(sheets)),
            Err(err) => {
                warnings.push(format!("表格文本提取告警: {err}"));
                (None, None)
            }
        },
        _ => (None, None),
    };

    let cleaned = extracted
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(BookDocumentInspection {
        format_label,
        extracted_text: cleaned,
        page_count,
        warnings,
    })
}

/// Build a default `BOOK.md` document for a newly imported Book document.
#[must_use]
pub fn build_imported_book_markdown(
    book_name: &str,
    description: Option<&str>,
    file_name: &str,
    stored_relative_path: &str,
    note_relative_path: Option<&str>,
    inspection: &BookDocumentInspection,
) -> String {
    let summary = description
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(
            || format!("由文档 {file_name} 导入生成的知识书籍"),
            str::to_string,
        );

    let mut markdown = format!(
        "---\nname: {book_name}\ndescription: {summary}\n---\n\n# {book_name}\n\n{summary}\n\n## 导入文档\n\n- 原始文件: `{stored_relative_path}`\n- 文档格式: `{}`\n",
        inspection.format_label
    );

    if let Some(pages) = inspection.page_count {
        let _ = writeln!(markdown, "- 页数/工作表数: `{pages}`");
    }
    if let Some(note_path) = note_relative_path {
        let _ = writeln!(markdown, "- 提取文本归档: `{note_path}`");
    }
    markdown
}

/// Render a Markdown note file containing the extracted plain text of an imported document.
#[must_use]
pub fn render_generated_document_note(
    file_name: &str,
    stored_relative_path: &str,
    source_label: &str,
    inspection: &BookDocumentInspection,
    extracted_text: &str,
) -> String {
    let mut note = format!(
        "# {file_name} 文本提取归档\n\n- 原始文件: `{stored_relative_path}`\n- 导入来源: `{source_label}`\n- 文档格式: `{}`\n",
        inspection.format_label
    );
    if let Some(pages) = inspection.page_count {
        let _ = writeln!(note, "- 页数/工作表数: `{pages}`");
    }
    note.push_str("\n## 提取正文\n\n");
    note.push_str(extracted_text.trim());
    note.push('\n');
    note
}

fn extract_pdf_text(bytes: &[u8]) -> Result<(String, usize), String> {
    let doc = pdf_oxide::PdfDocument::from_bytes(bytes.to_vec()).map_err(|e| e.to_string())?;
    let page_count = doc.page_count().map_err(|e| e.to_string())?;
    let mut out = String::new();
    for page_idx in 0..page_count {
        if let Ok(page_text) = doc.extract_text(page_idx) {
            let trimmed = page_text.trim();
            if !trimmed.is_empty() {
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(trimmed);
            }
        }
    }
    Ok((out, page_count))
}

fn extract_docx_text(bytes: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let mut doc_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|e| e.to_string())?
        .read_to_string(&mut doc_xml)
        .map_err(|e| e.to_string())?;
    Ok(extract_xml_text_nodes(&doc_xml, b"w:p"))
}

fn extract_pptx_text(bytes: &[u8]) -> Result<(String, usize), String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let mut slide_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .and_then(|f| f.name().ok().map(std::borrow::Cow::into_owned))
        })
        .filter(|name| {
            name.starts_with("ppt/slides/slide")
                && Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
        })
        .collect();
    slide_names.sort();

    let mut out = String::new();
    let count = slide_names.len();
    for slide_name in slide_names {
        let mut xml = String::new();
        if let Ok(mut file) = archive.by_name(&slide_name)
            && file.read_to_string(&mut xml).is_ok()
        {
            let slide_text = extract_xml_text_nodes(&xml, b"a:p");
            let trimmed = slide_text.trim();
            if !trimmed.is_empty() {
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(trimmed);
            }
        }
    }
    Ok((out, count))
}

fn extract_xml_text_nodes(xml: &str, paragraph_tag: &[u8]) -> String {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut out = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) => {
                if let Ok(decoded) = text.xml10_content() {
                    out.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(entity)) => {
                if let Ok(Some(ch)) = entity.resolve_char_ref() {
                    out.push(ch);
                } else if let Ok(name) = entity.decode()
                    && let Some(resolved) = quick_xml::escape::resolve_predefined_entity(&name)
                {
                    out.push_str(resolved);
                }
            }
            Ok(Event::End(end)) if end.name().as_ref() == paragraph_tag => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    out.trim().to_string()
}

fn extract_spreadsheet_text(bytes: &[u8]) -> Result<(String, usize), String> {
    let cursor = Cursor::new(bytes);
    let mut workbook: Sheets<Cursor<&[u8]>> =
        open_workbook_auto_from_rs(cursor).map_err(|e| e.to_string())?;
    let sheet_names = workbook.sheet_names();
    let sheet_count = sheet_names.len();
    let mut out = String::new();

    for sheet_name in sheet_names {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            let _ = writeln!(out, "## Sheet: {sheet_name}");
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(format_spreadsheet_cell)
                    .filter(|s| !s.is_empty())
                    .collect();
                if !cells.is_empty() {
                    out.push_str(&cells.join("\t"));
                    out.push('\n');
                }
            }
        }
    }

    Ok((out.trim().to_string(), sheet_count))
}

fn format_spreadsheet_cell(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.trim().to_string(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::Error(e) => format!("{e:?}"),
    }
}
