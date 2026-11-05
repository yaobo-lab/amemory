//! Book 目录扫描、Memvid 同步与精确 Markdown 资源读取。

use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use eyre::Context;
use memvid_core::{
    DocMetadata, DocumentFormat, EmbeddingProvider, FrameStatus, Memvid, PutOptions, ReaderHint,
    ReaderRegistry,
};
use serde::{Deserialize, Serialize};

const CATALOG_URI: &str = "books://catalog";
const BOOK_TRACK: &str = "books";
const BOOK_ENTRY_FILE: &str = "BOOK.md";
const BOOK_SOURCES_DIR: &str = "sources";
const MAX_PART_CHARS: usize = 1_800;
const MAX_BOOK_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const DOCUMENT_EMBEDDER_KEY: &str = "book_embedding_provider";
const DOCUMENT_HASH_KEY: &str = "book_source_blake3";

/// 当前 Book 目录的内存目录。
#[derive(Clone, Debug)]
pub struct BookCatalog {
    root: PathBuf,
    max_document_bytes: usize,
    books: BTreeMap<String, BookDefinition>,
}

/// 一个可通过 MCP 加载的 Markdown Book。
#[derive(Clone, Debug)]
pub struct BookDefinition {
    name: String,
    title: String,
    description: Option<String>,
    author: Option<String>,
    entry_file: String,
    files: BTreeMap<String, BookFile>,
    documents: BTreeMap<String, BookDocument>,
}

#[derive(Clone, Debug)]
struct BookFile {
    content: String,
    part_count: usize,
}

#[derive(Clone, Debug)]
struct BookDocument {
    full_path: PathBuf,
    kind: BookDocumentKind,
    size: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BookDocumentKind {
    Pdf,
    Docx,
    Pptx,
    Xls,
    Xlsx,
}

impl BookDocumentKind {
    const fn mime(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Xls => "application/vnd.ms-excel",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        }
    }

    const fn format(self) -> DocumentFormat {
        match self {
            Self::Pdf => DocumentFormat::Pdf,
            Self::Docx => DocumentFormat::Docx,
            Self::Pptx => DocumentFormat::Pptx,
            Self::Xls => DocumentFormat::Xls,
            Self::Xlsx => DocumentFormat::Xlsx,
        }
    }
}

/// Reader 对导入文档执行完整文本提取后的检查结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookDocumentInspection {
    /// 导入后使用的安全 leaf 文件名。
    pub file_name: String,
    /// 根据扩展名确定的 MIME 类型。
    pub mime: String,
    /// 原始文档字节数。
    pub bytes: usize,
    /// 实际执行提取的 Memvid reader 名称。
    pub reader_name: String,
    /// 提取全文的 Unicode 字符数。
    pub extracted_chars: usize,
    /// Reader 返回的非致命警告。
    pub warnings: Vec<String>,
    /// Reader 是否使用了回退提取路径。
    pub fallback: bool,
    /// Reader 报告的处理页数。
    pub pages_processed: Option<u32>,
    /// Reader 报告的提取耗时（毫秒）。
    pub duration_ms: Option<u64>,
}

/// 从白名单目录安全读取、可直接保存的本地文档。
#[allow(
    dead_code,
    reason = "public document import API is integrated by the MCP layer"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBookDocument {
    /// 源路径的 UTF-8 leaf 文件名。
    pub file_name: String,
    /// 在大小限制内读取的完整原始内容。
    pub bytes: Vec<u8>,
}

/// 将 Book 目录内容同步到 Memvid 后的统计信息。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BookSyncReport {
    /// 新增或更新的 Markdown 资源分块数。
    pub written_parts: usize,
    /// 内容和元数据均未变化的 Markdown 资源分块数。
    pub unchanged_parts: usize,
    /// 因源文件或 Book 已删除而移除的分块数。
    pub deleted_parts: usize,
    /// 新增或替换的 PDF/Office 文档数。
    pub written_documents: usize,
    /// 内容和嵌入配置均未变化的 PDF/Office 文档数。
    pub unchanged_documents: usize,
    /// 因源文件或 Book 已删除而移除的 PDF/Office 文档数。
    pub deleted_documents: usize,
    /// 当前可用的 Book 数量。
    pub book_count: usize,
}

/// 创建、更新或删除磁盘 Book 时发生的错误。
#[derive(Debug)]
pub struct BookMutationError {
    kind: BookMutationErrorKind,
    message: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BookMutationErrorKind {
    InvalidRequest,
    FileSystem,
}

impl BookMutationError {
    /// 返回该错误是否由调用参数或目标 Book 状态引起。
    #[must_use]
    pub const fn is_request_error(&self) -> bool {
        matches!(self.kind, BookMutationErrorKind::InvalidRequest)
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self {
            kind: BookMutationErrorKind::InvalidRequest,
            message: message.into(),
            source: None,
        }
    }

    fn file_system(operation: &str, path: &Path, source: io::Error) -> Self {
        Self {
            kind: BookMutationErrorKind::FileSystem,
            message: format!("{operation} {}", path.display()),
            source: Some(Box::new(source)),
        }
    }

    fn document(operation: impl Into<String>, source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            kind: BookMutationErrorKind::InvalidRequest,
            message: operation.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for BookMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            write!(formatter, "{}: {source}", self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl Error for BookMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as _)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct StoredCatalog {
    version: u32,
    books: BTreeMap<String, StoredBook>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoredBook {
    title: String,
    description: Option<String>,
    author: Option<String>,
    entry_file: String,
    files: BTreeMap<String, usize>,
    #[serde(default)]
    file_blake3: BTreeMap<String, String>,
    #[serde(default)]
    documents: BTreeMap<String, String>,
}

#[derive(Debug)]
struct BookMetadata {
    title: String,
    description: Option<String>,
    author: Option<String>,
}

impl BookCatalog {
    /// 扫描根目录的直接子目录，并将每个子目录解释为一个 Book。
    ///
    /// 空根目录是合法目录。每个 Book 必须包含常规文件 `BOOK.md`；扫描器会递归收集
    /// Book 目录中所有 UTF-8 Markdown 文件，并忽略其他常规文件。
    ///
    /// # Errors
    ///
    /// 根目录不可读取、Book 名称或 `BOOK.md` 无效、目录内存在符号链接，或者文件系统
    /// 操作失败时返回错误。
    pub fn scan(root: impl AsRef<Path>, max_document_bytes: usize) -> eyre::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut directories = fs::read_dir(&root)
            .wrap_err_with(|| format!("reading books directory {}", root.display()))?
            .collect::<Result<Vec<_>, _>>()?;
        directories.sort_by_key(fs::DirEntry::file_name);

        let mut books = BTreeMap::new();
        for directory in directories {
            let file_type = directory.file_type()?;
            if file_type.is_symlink() {
                eyre::bail!(
                    "book root entry must not be a symbolic link: {}",
                    directory.path().display()
                );
            }
            if !file_type.is_dir() {
                continue;
            }

            let definition = scan_book_directory(&directory.path(), max_document_bytes)?;
            let name = definition.name.clone();
            if books.insert(name.clone(), definition).is_some() {
                eyre::bail!("duplicate book name: {name}");
            }
        }

        Ok(Self {
            root,
            max_document_bytes,
            books,
        })
    }

    /// 返回 Book 根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 返回单个导入文档允许的最大字节数。
    #[must_use]
    pub const fn max_document_bytes(&self) -> usize {
        self.max_document_bytes
    }

    /// 按稳定名称查找 Book。
    #[must_use]
    pub fn book(&self, name: &str) -> Option<&BookDefinition> {
        self.books.get(name)
    }

    /// 按名称排序迭代所有 Book。
    pub fn iter(&self) -> impl Iterator<Item = &BookDefinition> {
        self.books.values()
    }
}

impl BookDefinition {
    /// 返回 Book 的稳定名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回 Book 的非空标题。
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// 返回 Book 的可选说明。
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// 返回 Book 的可选作者。
    #[must_use]
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// 返回主 Markdown 文件相对于 Book 目录的固定路径。
    #[must_use]
    pub fn entry_file(&self) -> &str {
        &self.entry_file
    }

    /// 按路径排序迭代 Book 中的 UTF-8 Markdown 文件。
    pub fn file_paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// 按路径排序迭代已导入的 PDF 和 Office 源文档。
    pub fn document_paths(&self) -> impl Iterator<Item = &str> {
        self.documents.keys().map(String::as_str)
    }

    /// 返回指定 Markdown 文件的稳定分块数量。
    #[must_use]
    pub fn part_count(&self, path: &str) -> Option<usize> {
        self.files.get(path).map(|file| file.part_count)
    }
}

/// 在 Book 根目录中创建一个包含 `BOOK.md` 的新 Book。
///
/// # Errors
///
/// 名称或文档无效、目标目录已经存在，或者目录和文件无法创建时返回错误。
pub fn create_book(root: &Path, name: &str, content: &str) -> Result<PathBuf, BookMutationError> {
    validate_book_name(name)?;
    validate_book_document(name, content)?;

    let directory = root.join(name);
    match fs::symlink_metadata(&directory) {
        Ok(_) => {
            return Err(BookMutationError::invalid(format!(
                "book already exists: {name}"
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(BookMutationError::file_system(
                "checking book path",
                &directory,
                error,
            ));
        }
    }

    fs::create_dir(&directory).map_err(|error| {
        BookMutationError::file_system("creating book directory", &directory, error)
    })?;
    let entry = directory.join(BOOK_ENTRY_FILE);
    if let Err(error) = fs::write(&entry, content) {
        if let Err(cleanup_error) = fs::remove_dir(&directory) {
            log::warn!(
                "failed to clean up incomplete book directory: path={}, error={cleanup_error}",
                directory.display()
            );
        }
        return Err(BookMutationError::file_system(
            "writing book entry",
            &entry,
            error,
        ));
    }

    Ok(directory)
}

/// 完整替换现有 Book 的 `BOOK.md`，并保留章节等其他文件。
///
/// # Errors
///
/// 名称或文档无效、目标 Book 不存在、目录或入口是符号链接，或者主文件无法写入时
/// 返回错误。
pub fn update_book(root: &Path, name: &str, content: &str) -> Result<PathBuf, BookMutationError> {
    validate_book_name(name)?;
    validate_book_document(name, content)?;

    let directory = existing_book_directory(root, name)?;
    let entry = directory.join(BOOK_ENTRY_FILE);
    let metadata = fs::symlink_metadata(&entry).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            BookMutationError::invalid(format!("book has no {BOOK_ENTRY_FILE} entry: {name}"))
        } else {
            BookMutationError::file_system("checking book entry", &entry, error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(BookMutationError::invalid(format!(
            "book entry is not a regular file: {}",
            entry.display()
        )));
    }

    fs::write(&entry, content)
        .map_err(|error| BookMutationError::file_system("writing book entry", &entry, error))?;
    Ok(directory)
}

/// 校验文档名是受支持格式的安全 leaf 文件名。
///
/// # Errors
///
/// 文件名为空、包含目录或控制字符、过长，或者扩展名不在白名单中时返回错误。
pub fn validate_document_file_name(file_name: &str) -> Result<(), BookMutationError> {
    validated_document_kind(file_name).map(|_| ())
}

/// 使用默认 Memvid reader 完整检查一个待导入文档。
///
/// 检查会验证安全文件名、扩展名、字节限制，并要求 reader 提取到非空文本。
///
/// # Errors
///
/// 文件名或大小无效、没有匹配 reader、reader 提取失败，或文档没有可提取文本时返回错误。
#[allow(
    dead_code,
    reason = "public document import API is integrated by the MCP layer"
)]
pub fn inspect_document(
    file_name: &str,
    bytes: &[u8],
    max_document_bytes: usize,
) -> Result<BookDocumentInspection, BookMutationError> {
    inspect_document_with_text(file_name, bytes, max_document_bytes)
        .map(|(inspection, _)| inspection)
}

fn inspect_document_with_text(
    file_name: &str,
    bytes: &[u8],
    max_document_bytes: usize,
) -> Result<(BookDocumentInspection, String), BookMutationError> {
    let kind = validated_document_kind(file_name)?;
    validate_document_size(bytes.len(), max_document_bytes)?;

    let registry = ReaderRegistry::default();
    let hint = ReaderHint::new(Some(kind.mime()), Some(kind.format()))
        .with_uri(Some(file_name))
        .with_magic(Some(bytes));
    let reader = registry.find_reader(&hint).ok_or_else(|| {
        BookMutationError::invalid(format!(
            "no Memvid reader accepted document '{file_name}' ({})",
            kind.mime()
        ))
    })?;
    let output = reader.extract(bytes, &hint).map_err(|error| {
        BookMutationError::document(
            format!("extracting document '{file_name}' with {}", reader.name()),
            error,
        )
    })?;
    let extracted_text = output
        .document
        .text
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| {
            BookMutationError::invalid(format!(
                "document contains no extractable text: {file_name}"
            ))
        })?;
    let extracted_chars = extracted_text.chars().count();
    let inspection = BookDocumentInspection {
        file_name: file_name.to_owned(),
        mime: kind.mime().to_owned(),
        bytes: bytes.len(),
        reader_name: output.reader_name,
        extracted_chars,
        warnings: output.diagnostics.warnings,
        fallback: output.diagnostics.fallback,
        pages_processed: output.diagnostics.pages_processed,
        duration_ms: output.diagnostics.duration_ms,
    };
    Ok((inspection, extracted_text))
}

/// 从配置白名单内 canonicalize 并安全读取本地文档。
///
/// # Errors
///
/// 源路径或白名单根目录无法 canonicalize、源不在任何白名单根目录内、源不是常规文件、
/// 文件名或格式无效、文件超过限制，或者读取失败时返回错误。
#[allow(
    dead_code,
    reason = "public document import API is integrated by the MCP layer"
)]
pub fn read_local_document(
    source_path: &Path,
    allowed_roots: &[PathBuf],
    max_document_bytes: usize,
) -> Result<LocalBookDocument, BookMutationError> {
    let source_metadata = fs::symlink_metadata(source_path).map_err(|error| {
        BookMutationError::file_system("checking document source", source_path, error)
    })?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        return Err(BookMutationError::invalid(format!(
            "document source must be a regular non-symlink file: {}",
            source_path.display()
        )));
    }
    let canonical_source = fs::canonicalize(source_path).map_err(|error| {
        BookMutationError::file_system("canonicalizing document source", source_path, error)
    })?;

    let mut canonical_roots = Vec::with_capacity(allowed_roots.len());
    for root in allowed_roots {
        let canonical_root = fs::canonicalize(root).map_err(|error| {
            BookMutationError::file_system("canonicalizing allowed document root", root, error)
        })?;
        let metadata = fs::metadata(&canonical_root).map_err(|error| {
            BookMutationError::file_system("checking allowed document root", &canonical_root, error)
        })?;
        if !metadata.is_dir() {
            return Err(BookMutationError::invalid(format!(
                "allowed document root is not a directory: {}",
                canonical_root.display()
            )));
        }
        canonical_roots.push(canonical_root);
    }
    if !canonical_roots
        .iter()
        .any(|root| canonical_source.starts_with(root))
    {
        return Err(BookMutationError::invalid(format!(
            "document source is outside allowed roots: {}",
            canonical_source.display()
        )));
    }

    let metadata = fs::metadata(&canonical_source).map_err(|error| {
        BookMutationError::file_system("checking document source", &canonical_source, error)
    })?;
    if !metadata.is_file() {
        return Err(BookMutationError::invalid(format!(
            "document source is not a regular file: {}",
            canonical_source.display()
        )));
    }
    let size = usize::try_from(metadata.len()).map_err(|_| {
        BookMutationError::invalid(format!(
            "document is too large for this platform: {}",
            canonical_source.display()
        ))
    })?;
    validate_document_size(size, max_document_bytes)?;

    let file_name = canonical_source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BookMutationError::invalid("document file name must be valid UTF-8"))?
        .to_owned();
    validate_document_file_name(&file_name)?;
    let bytes = fs::read(&canonical_source).map_err(|error| {
        BookMutationError::file_system("reading document source", &canonical_source, error)
    })?;
    validate_document_size(bytes.len(), max_document_bytes)?;

    Ok(LocalBookDocument { file_name, bytes })
}

/// 将 PDF 或 Office 文档保存到现有 Book 的 `sources/` 目录。
///
/// # Errors
///
/// Book 或文件名无效、格式不受支持、内容为空或超过限制，或者文件系统操作失败时返回错误。
pub fn store_document(
    root: &Path,
    book_name: &str,
    file_name: &str,
    bytes: &[u8],
    max_document_bytes: usize,
) -> Result<PathBuf, BookMutationError> {
    validate_book_name(book_name)?;
    validate_document_file_name(file_name)?;
    validate_document_size(bytes.len(), max_document_bytes)?;

    let directory = existing_book_directory(root, book_name)?;
    let sources = directory.join(BOOK_SOURCES_DIR);
    match fs::symlink_metadata(&sources) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(BookMutationError::invalid(format!(
                "book sources path is not a regular directory: {}",
                sources.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(&sources).map_err(|error| {
                BookMutationError::file_system("creating book sources directory", &sources, error)
            })?;
        }
        Err(error) => {
            return Err(BookMutationError::file_system(
                "checking book sources directory",
                &sources,
                error,
            ));
        }
    }

    let destination = sources.join(file_name);
    if let Ok(metadata) = fs::symlink_metadata(&destination)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(BookMutationError::invalid(format!(
            "document destination is not a regular file: {}",
            destination.display()
        )));
    }
    fs::write(&destination, bytes).map_err(|error| {
        BookMutationError::file_system("writing book source document", &destination, error)
    })?;
    Ok(destination)
}

/// 永久删除一个 Book 目录及其全部资源文件。
///
/// # Errors
///
/// 名称无效、目标 Book 不存在、目录是符号链接，或者目录无法删除时返回错误。
pub fn delete_book(root: &Path, name: &str) -> Result<PathBuf, BookMutationError> {
    validate_book_name(name)?;
    let directory = existing_book_directory(root, name)?;
    fs::remove_dir_all(&directory).map_err(|error| {
        BookMutationError::file_system("deleting book directory", &directory, error)
    })?;
    Ok(directory)
}

fn existing_book_directory(root: &Path, name: &str) -> Result<PathBuf, BookMutationError> {
    let directory = root.join(name);
    let metadata = fs::symlink_metadata(&directory).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            BookMutationError::invalid(format!("unknown book: {name}"))
        } else {
            BookMutationError::file_system("checking book directory", &directory, error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(BookMutationError::invalid(format!(
            "book path is not a regular directory: {}",
            directory.display()
        )));
    }
    Ok(directory)
}

fn validate_book_name(name: &str) -> Result<(), BookMutationError> {
    let bytes = name.as_bytes();
    let is_alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !(1..=64).contains(&bytes.len())
        || !bytes.first().copied().is_some_and(is_alphanumeric)
        || !bytes.last().copied().is_some_and(is_alphanumeric)
        || bytes
            .iter()
            .any(|byte| !is_alphanumeric(*byte) && *byte != b'-')
        || name.contains("--")
    {
        return Err(BookMutationError::invalid(
            "name must contain at most 64 lowercase ASCII letters or digits separated by single hyphens",
        ));
    }
    Ok(())
}

fn validated_document_kind(file_name: &str) -> Result<BookDocumentKind, BookMutationError> {
    let path = Path::new(file_name);
    if file_name.is_empty()
        || file_name.len() > 255
        || file_name.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
        || file_name.ends_with(' ')
        || file_name.ends_with('.')
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name)
    {
        return Err(BookMutationError::invalid(
            "file_name must be a safe plain file name without directories, control characters, or reserved characters",
        ));
    }
    document_kind(path).ok_or_else(|| {
        BookMutationError::invalid(
            "supported document extensions are pdf, docx, pptx, xls, and xlsx",
        )
    })
}

fn validate_document_size(size: usize, max_document_bytes: usize) -> Result<(), BookMutationError> {
    if size == 0 {
        return Err(BookMutationError::invalid("document must not be empty"));
    }
    if size > max_document_bytes {
        return Err(BookMutationError::invalid(format!(
            "document exceeds the {max_document_bytes}-byte limit"
        )));
    }
    Ok(())
}

fn document_kind(path: &Path) -> Option<BookDocumentKind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "pdf" => Some(BookDocumentKind::Pdf),
        "docx" => Some(BookDocumentKind::Docx),
        "pptx" => Some(BookDocumentKind::Pptx),
        "xls" => Some(BookDocumentKind::Xls),
        "xlsx" => Some(BookDocumentKind::Xlsx),
        _ => None,
    }
}

fn validate_book_document(name: &str, content: &str) -> Result<BookMetadata, BookMutationError> {
    if content.len() > MAX_BOOK_DOCUMENT_BYTES {
        return Err(BookMutationError::invalid(format!(
            "content exceeds the {MAX_BOOK_DOCUMENT_BYTES}-byte limit"
        )));
    }
    if content.contains('\0') {
        return Err(BookMutationError::invalid(
            "content must not contain NUL characters",
        ));
    }

    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err(BookMutationError::invalid(
            "content must start with a YAML frontmatter delimiter",
        ));
    }

    let mut metadata = BTreeMap::new();
    let mut closed = false;
    for line in &mut lines {
        let line = line.trim();
        if line == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            metadata.insert(
                key.trim().to_owned(),
                value.trim().trim_matches(['\'', '"']).to_owned(),
            );
        }
    }
    if !closed {
        return Err(BookMutationError::invalid(
            "content must close its YAML frontmatter with ---",
        ));
    }

    let document_name = metadata
        .get("name")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BookMutationError::invalid("frontmatter.name is required"))?;
    if document_name != name {
        return Err(BookMutationError::invalid(format!(
            "frontmatter.name must match the requested name '{name}'"
        )));
    }

    let title = metadata
        .get("title")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BookMutationError::invalid("frontmatter.title is required"))?
        .trim()
        .to_owned();
    if !lines.any(|line| !line.trim().is_empty()) {
        return Err(BookMutationError::invalid(
            "content must include a non-empty Markdown body",
        ));
    }

    Ok(BookMetadata {
        title,
        description: optional_metadata(&metadata, "description"),
        author: optional_metadata(&metadata, "author"),
    })
}

fn optional_metadata(metadata: &BTreeMap<String, String>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// 将当前 Book 目录增量同步到独立的 Memvid Book track。
///
/// 所有资源使用 `book://` URI，目录使用 `books://catalog` URI。资源发生变化时先提交父子帧，
/// 再单独更新目录，避免大量文档 chunk 触发的自动 checkpoint 与目录更新相互覆盖。
///
/// # Errors
///
/// Memvid 帧读取、写入、删除、序列化或提交失败时返回错误。
pub fn sync_to_memvid(
    memvid: &mut Memvid,
    catalog: &BookCatalog,
    embedder: Option<&dyn EmbeddingProvider>,
) -> eyre::Result<BookSyncReport> {
    let previous = read_stored_catalog(memvid)?;
    sync_to_memvid_with_previous(memvid, catalog, &previous, embedder)
}

/// 使用服务内存中的上一版 Book 目录执行增量同步。
///
/// 运行期使用此入口可避免在同一个 Memvid 实例中读取刚经过 checkpoint 的目录帧；进程启动
/// 时仍应使用 [`sync_to_memvid`] 从持久化目录恢复状态。
///
/// # Errors
///
/// 文档读取、Memvid 帧写入、删除、序列化或提交失败时返回错误。
pub fn sync_to_memvid_incremental(
    memvid: &mut Memvid,
    previous_catalog: &BookCatalog,
    catalog: &BookCatalog,
    embedder: Option<&dyn EmbeddingProvider>,
) -> eyre::Result<BookSyncReport> {
    let previous = StoredCatalog::from(previous_catalog);
    sync_to_memvid_with_previous(memvid, catalog, &previous, embedder)
}

fn sync_to_memvid_with_previous(
    memvid: &mut Memvid,
    catalog: &BookCatalog,
    previous: &StoredCatalog,
    embedder: Option<&dyn EmbeddingProvider>,
) -> eyre::Result<BookSyncReport> {
    let current = StoredCatalog::from(catalog);
    let mut report = BookSyncReport {
        book_count: catalog.books.len(),
        ..BookSyncReport::default()
    };

    for book in catalog.books.values() {
        let metadata_changed = previous.books.get(&book.name).is_some_and(|old| {
            old.title != book.title
                || old.description != book.description
                || old.author != book.author
                || old.entry_file != book.entry_file
        });

        for (path, file) in &book.files {
            let content_hash = blake3::hash(file.content.as_bytes()).to_hex().to_string();
            let file_unchanged = !metadata_changed
                && previous
                    .books
                    .get(&book.name)
                    .and_then(|old| old.file_blake3.get(path))
                    .is_some_and(|old_hash| old_hash == &content_hash);
            for (index, part) in split_text(&file.content).iter().enumerate() {
                let uri = part_uri(&book.name, path, index);
                let frame_unchanged = file_unchanged
                    && memvid
                        .frame_by_uri(&uri)
                        .is_ok_and(|frame| frame.status == FrameStatus::Active);
                if frame_unchanged {
                    report.unchanged_parts += 1;
                    continue;
                }

                let options = book_part_options(book, path, part);
                if upsert_frame(memvid, &uri, part.as_bytes(), options)? {
                    report.written_parts += 1;
                } else {
                    report.unchanged_parts += 1;
                }
            }
        }
    }

    for book in catalog.books.values() {
        for (path, document) in &book.documents {
            let bytes = read_scanned_document(document, catalog.max_document_bytes)?;
            let uri = document_uri(&book.name, path);
            if upsert_document(memvid, book, path, document, &bytes, &uri, embedder)? {
                report.written_documents += 1;
            } else {
                report.unchanged_documents += 1;
            }
        }
    }

    for (old_name, old_book) in &previous.books {
        let current_book = current.books.get(old_name);
        for (old_path, old_part_count) in &old_book.files {
            let current_part_count = current_book
                .and_then(|book| book.files.get(old_path))
                .copied()
                .unwrap_or(0);
            for index in current_part_count..*old_part_count {
                if delete_active_frame(memvid, &part_uri(old_name, old_path, index))? {
                    report.deleted_parts += 1;
                }
            }
        }
        for old_path in old_book.documents.keys() {
            let still_exists =
                current_book.is_some_and(|book| book.documents.contains_key(old_path));
            if !still_exists && delete_document_frame(memvid, &document_uri(old_name, old_path))? {
                report.deleted_documents += 1;
            }
        }
    }

    let resources_changed = report.written_parts > 0
        || report.deleted_parts > 0
        || report.written_documents > 0
        || report.deleted_documents > 0;
    if resources_changed {
        memvid.commit().wrap_err("committing book resources")?;
    }

    let serialized = serde_json::to_vec_pretty(&current)?;
    let catalog_options = PutOptions::builder()
        .track(BOOK_TRACK)
        .kind("book-catalog")
        .uri(CATALOG_URI)
        .title("Book catalog")
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .build();
    let catalog_changed = if current == *previous {
        false
    } else {
        replace_frame(memvid, CATALOG_URI, &serialized, catalog_options)?
    };

    if catalog_changed {
        memvid.commit().wrap_err("committing book catalog")?;
    }

    Ok(report)
}

/// 从 Memvid 中读取并拼接一个 Book Markdown 资源的完整内容。
///
/// 省略 `resource` 时加载固定入口 `BOOK.md`。
///
/// # Errors
///
/// Book 或资源不存在、对应帧不是活动帧、帧无法读取，或帧内容不是 UTF-8 时返回错误。
pub fn load_resource(
    memvid: &mut Memvid,
    catalog: &BookCatalog,
    book_name: &str,
    resource: Option<&str>,
) -> eyre::Result<(String, String)> {
    let book = catalog
        .book(book_name)
        .ok_or_else(|| eyre::eyre!("unknown book: {book_name}"))?;
    let path = resource.unwrap_or_else(|| book.entry_file());
    let part_count = book
        .part_count(path)
        .ok_or_else(|| eyre::eyre!("book '{}' has no resource '{path}'", book.name()))?;

    let mut content = String::new();
    for index in 0..part_count {
        let uri = part_uri(book.name(), path, index);
        let frame = memvid
            .frame_by_uri(&uri)
            .wrap_err_with(|| format!("finding Memvid frame {uri}"))?;
        if frame.status != FrameStatus::Active {
            eyre::bail!("Memvid frame is not active: {uri}");
        }
        let bytes = memvid
            .frame_canonical_payload(frame.id)
            .wrap_err_with(|| format!("reading Memvid frame {uri}"))?;
        let part = String::from_utf8(bytes)
            .wrap_err_with(|| format!("book resource is not UTF-8: {uri}"))?;
        content.push_str(&part);
    }

    Ok((path.to_owned(), content))
}

fn scan_book_directory(
    directory: &Path,
    max_document_bytes: usize,
) -> eyre::Result<BookDefinition> {
    let directory_metadata = fs::symlink_metadata(directory)
        .wrap_err_with(|| format!("checking book directory {}", directory.display()))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        eyre::bail!(
            "book path is not a regular directory: {}",
            directory.display()
        );
    }

    let directory_name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre::eyre!("invalid book directory name: {}", directory.display()))?;
    validate_book_name(directory_name).map_err(eyre::Report::new)?;

    let entry_path = directory.join(BOOK_ENTRY_FILE);
    let entry_metadata = fs::symlink_metadata(&entry_path)
        .wrap_err_with(|| format!("checking book entry {}", entry_path.display()))?;
    if entry_metadata.file_type().is_symlink() || !entry_metadata.is_file() {
        eyre::bail!("book entry is not a regular file: {}", entry_path.display());
    }

    let entry_content = fs::read_to_string(&entry_path)
        .wrap_err_with(|| format!("reading book entry {}", entry_path.display()))?;
    let metadata =
        validate_book_document(directory_name, &entry_content).map_err(eyre::Report::new)?;

    let mut paths = Vec::new();
    collect_markdown_files(directory, directory, &mut paths)?;
    paths.sort();

    let mut files = BTreeMap::new();
    for relative_path in paths {
        let full_path = directory.join(Path::new(&relative_path));
        let bytes = fs::read(&full_path)
            .wrap_err_with(|| format!("reading book resource {}", full_path.display()))?;
        let Ok(content) = String::from_utf8(bytes) else {
            log::warn!(
                "skipping non-UTF-8 book resource: path={}",
                full_path.display()
            );
            continue;
        };
        let part_count = split_text(&content).len();
        files.insert(
            relative_path,
            BookFile {
                content,
                part_count,
            },
        );
    }

    if !files.contains_key(BOOK_ENTRY_FILE) {
        eyre::bail!("book entry is not UTF-8: {}", entry_path.display());
    }
    let documents = collect_book_documents(directory, max_document_bytes)?;

    Ok(BookDefinition {
        name: directory_name.to_owned(),
        title: metadata.title,
        description: metadata.description,
        author: metadata.author,
        entry_file: BOOK_ENTRY_FILE.to_owned(),
        files,
        documents,
    })
}

fn collect_book_documents(
    book_directory: &Path,
    max_document_bytes: usize,
) -> eyre::Result<BTreeMap<String, BookDocument>> {
    let sources = book_directory.join(BOOK_SOURCES_DIR);
    let metadata = match fs::symlink_metadata(&sources) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(error).wrap_err_with(|| format!("checking {}", sources.display()));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        eyre::bail!(
            "book sources path is not a regular directory: {}",
            sources.display()
        );
    }

    let mut entries = fs::read_dir(&sources)
        .wrap_err_with(|| format!("reading book sources directory {}", sources.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);

    let mut documents = BTreeMap::new();
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            eyre::bail!(
                "book sources must not contain symbolic links: {}",
                path.display()
            );
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(kind) = document_kind(&path) else {
            continue;
        };
        let size = usize::try_from(entry.metadata()?.len()).wrap_err_with(|| {
            format!(
                "document is too large for this platform: {}",
                path.display()
            )
        })?;
        if size == 0 || size > max_document_bytes {
            eyre::bail!(
                "book source document must contain 1..={max_document_bytes} bytes: {}",
                path.display()
            );
        }
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| eyre::eyre!("book source file name is not UTF-8: {}", path.display()))?;
        let relative_path = format!("{BOOK_SOURCES_DIR}/{file_name}");
        documents.insert(
            relative_path,
            BookDocument {
                full_path: path,
                kind,
                size,
            },
        );
    }
    Ok(documents)
}

fn collect_markdown_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<String>,
) -> eyre::Result<()> {
    let mut entries = fs::read_dir(directory)
        .wrap_err_with(|| format!("reading book directory {}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            eyre::bail!(
                "book directory must not contain symbolic links: {}",
                path.display()
            );
        }
        if file_type.is_dir() {
            if directory == root && entry.file_name() == BOOK_SOURCES_DIR {
                continue;
            }
            collect_markdown_files(root, &path, output)?;
        } else if file_type.is_file() && is_markdown(&path) {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            output.push(relative);
        }
    }
    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn split_text(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![String::new()];
    }

    let mut parts = Vec::new();
    let mut start = 0;
    let mut char_count = 0;
    for (index, _) in content.char_indices() {
        if char_count == MAX_PART_CHARS {
            parts.push(content[start..index].to_owned());
            start = index;
            char_count = 0;
        }
        char_count += 1;
    }
    parts.push(content[start..].to_owned());
    parts
}

fn book_part_options(book: &BookDefinition, path: &str, part: &str) -> PutOptions {
    let metadata = DocMetadata {
        mime: Some("text/markdown".to_owned()),
        ..DocMetadata::default()
    };
    let description = book.description.as_deref().unwrap_or_default();
    let author = book.author.as_deref().unwrap_or_default();
    let search_text = format!(
        "book: {}\ntitle: {}\ndescription: {description}\nauthor: {author}\nresource: {path}\n{part}",
        book.name, book.title
    );

    PutOptions::builder()
        .track(BOOK_TRACK)
        .kind("book-resource")
        .uri(part_uri(&book.name, path, 0))
        .title(format!("{} — {path}", book.title))
        .metadata(metadata)
        .search_text(search_text)
        .push_tag("book")
        .push_tag(book.name.clone())
        .label("book")
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .build()
}

fn read_scanned_document(
    document: &BookDocument,
    max_document_bytes: usize,
) -> eyre::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(&document.full_path).wrap_err_with(|| {
        format!(
            "checking book source document {}",
            document.full_path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        eyre::bail!(
            "book source document is not a regular file: {}",
            document.full_path.display()
        );
    }
    let size = usize::try_from(metadata.len()).wrap_err_with(|| {
        format!(
            "document is too large for this platform: {}",
            document.full_path.display()
        )
    })?;
    if size == 0 || size > max_document_bytes {
        eyre::bail!(
            "book source document must contain 1..={max_document_bytes} bytes: {}",
            document.full_path.display()
        );
    }
    if size != document.size {
        eyre::bail!(
            "book source document changed after catalog scan: {}",
            document.full_path.display()
        );
    }

    let bytes = fs::read(&document.full_path).wrap_err_with(|| {
        format!(
            "reading book source document {}",
            document.full_path.display()
        )
    })?;
    if bytes.len() != size {
        eyre::bail!(
            "book source document changed while being read: {}",
            document.full_path.display()
        );
    }
    Ok(bytes)
}

fn upsert_document(
    memvid: &mut Memvid,
    book: &BookDefinition,
    path: &str,
    document: &BookDocument,
    bytes: &[u8],
    uri: &str,
    embedder: Option<&dyn EmbeddingProvider>,
) -> eyre::Result<bool> {
    let provider_label = embedder.map_or_else(
        || "none".to_owned(),
        |provider| format!("{}:{}", provider.kind(), provider.model()),
    );
    let content_hash = blake3::hash(bytes).to_hex().to_string();
    let existing = memvid
        .frame_by_uri(uri)
        .ok()
        .filter(|frame| frame.status == FrameStatus::Active);
    if existing.as_ref().is_some_and(|frame| {
        frame.extra_metadata.get(DOCUMENT_EMBEDDER_KEY) == Some(&provider_label)
            && frame.extra_metadata.get(DOCUMENT_HASH_KEY) == Some(&content_hash)
    }) {
        return Ok(false);
    }

    let file_name = document
        .full_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre::eyre!("book document file name is not UTF-8: {path}"))?;
    let (inspection, text) =
        inspect_document_with_text(file_name, bytes, bytes.len()).map_err(eyre::Report::new)?;
    for warning in &inspection.warnings {
        log::warn!(
            "book document reader warning: book={}, document={}, reader={}, warning={warning}",
            book.name,
            path,
            inspection.reader_name
        );
    }
    let reader_name = inspection.reader_name;

    let prepared_embeddings = if let Some(provider) = embedder {
        let parent = provider
            .embed_text(&text)
            .wrap_err_with(|| format!("embedding book document '{path}'"))?;
        let chunks = if let Some(chunks) = memvid.preview_chunks(text.as_bytes()) {
            let chunk_texts: Vec<&str> = chunks.iter().map(String::as_str).collect();
            Some(
                provider
                    .embed_batch(&chunk_texts)
                    .wrap_err_with(|| format!("embedding chunks for book document '{path}'"))?,
            )
        } else {
            None
        };
        Some((parent, chunks))
    } else {
        None
    };

    let metadata = DocMetadata {
        mime: Some(document.kind.mime().to_owned()),
        bytes: Some(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
        ..DocMetadata::default()
    };
    let mut options = PutOptions::builder()
        .track(BOOK_TRACK)
        .kind("book-document")
        .uri(uri)
        .title(format!("{} — {path}", book.title))
        .metadata(metadata)
        .push_tag("book")
        .push_tag(book.name.clone())
        .push_tag("book-document")
        .label("book")
        .auto_tag(true)
        .extract_dates(true)
        .extract_triplets(true)
        .instant_index(false)
        .extraction_budget_ms(0)
        .build();
    options
        .extra_metadata
        .insert(DOCUMENT_EMBEDDER_KEY.to_owned(), provider_label);
    options
        .extra_metadata
        .insert(DOCUMENT_HASH_KEY.to_owned(), content_hash);
    options
        .extra_metadata
        .insert("book_document_reader".to_owned(), reader_name);
    options
        .extra_metadata
        .insert("book_resource".to_owned(), path.to_owned());

    if existing.is_some() {
        delete_document_frame(memvid, uri)?;
    }
    match prepared_embeddings {
        Some((parent, Some(chunks))) => {
            memvid.put_with_chunk_embeddings(bytes, Some(parent), chunks, options)?;
        }
        Some((parent, None)) => {
            memvid.put_with_embedding_and_options(bytes, parent, options)?;
        }
        None => {
            memvid.put_bytes_with_options(bytes, options)?;
        }
    }
    Ok(true)
}

fn delete_document_frame(memvid: &mut Memvid, uri: &str) -> eyre::Result<bool> {
    let Ok(parent) = memvid.frame_by_uri(uri) else {
        return Ok(false);
    };
    if parent.status != FrameStatus::Active {
        return Ok(false);
    }

    let child_ids: Vec<u64> = (0..memvid.frame_count())
        .filter_map(|index| u64::try_from(index).ok())
        .filter_map(|frame_id| memvid.frame_by_id(frame_id).ok())
        .filter(|frame| frame.status == FrameStatus::Active && frame.parent_id == Some(parent.id))
        .map(|frame| frame.id)
        .collect();
    for child_id in child_ids {
        memvid.delete_frame(child_id)?;
    }
    memvid.delete_frame(parent.id)?;
    Ok(true)
}

fn replace_frame(
    memvid: &mut Memvid,
    uri: &str,
    bytes: &[u8],
    mut options: PutOptions,
) -> eyre::Result<bool> {
    options.uri = Some(uri.to_owned());
    delete_active_frame(memvid, uri)?;
    memvid.put_bytes_with_options(bytes, options)?;
    Ok(true)
}

fn upsert_frame(
    memvid: &mut Memvid,
    uri: &str,
    bytes: &[u8],
    mut options: PutOptions,
) -> eyre::Result<bool> {
    options.uri = Some(uri.to_owned());
    if let Ok(existing) = memvid.frame_by_uri(uri)
        && existing.status == FrameStatus::Active
    {
        memvid.update_frame(existing.id, Some(bytes.to_vec()), options, None)?;
        return Ok(true);
    }

    memvid.put_bytes_with_options(bytes, options)?;
    Ok(true)
}

fn delete_active_frame(memvid: &mut Memvid, uri: &str) -> eyre::Result<bool> {
    let Ok(frame) = memvid.frame_by_uri(uri) else {
        return Ok(false);
    };
    if frame.status != FrameStatus::Active {
        return Ok(false);
    }
    memvid.delete_frame(frame.id)?;
    Ok(true)
}

fn read_stored_catalog(memvid: &mut Memvid) -> eyre::Result<StoredCatalog> {
    let Ok(frame) = memvid.frame_by_uri(CATALOG_URI) else {
        return Ok(StoredCatalog::default());
    };
    if frame.status != FrameStatus::Active {
        return Ok(StoredCatalog::default());
    }

    let bytes = memvid.frame_canonical_payload(frame.id)?;
    serde_json::from_slice(&bytes).wrap_err("parsing stored book catalog")
}

impl From<&BookCatalog> for StoredCatalog {
    fn from(catalog: &BookCatalog) -> Self {
        let books = catalog
            .books
            .iter()
            .map(|(name, book)| {
                let files = book
                    .files
                    .iter()
                    .map(|(path, file)| (path.clone(), file.part_count))
                    .collect();
                let file_blake3 = book
                    .files
                    .iter()
                    .map(|(path, file)| {
                        (
                            path.clone(),
                            blake3::hash(file.content.as_bytes()).to_hex().to_string(),
                        )
                    })
                    .collect();
                let documents = book
                    .documents
                    .iter()
                    .map(|(path, document)| (path.clone(), document.kind.mime().to_owned()))
                    .collect();
                (
                    name.clone(),
                    StoredBook {
                        title: book.title.clone(),
                        description: book.description.clone(),
                        author: book.author.clone(),
                        entry_file: book.entry_file.clone(),
                        files,
                        file_blake3,
                        documents,
                    },
                )
            })
            .collect();
        Self { version: 3, books }
    }
}

fn document_uri(book_name: &str, path: &str) -> String {
    format!(
        "book://{}/documents/{}",
        percent_encode(book_name),
        percent_encode(path)
    )
}

fn part_uri(book_name: &str, path: &str, index: usize) -> String {
    format!("{}part-{index:05}", part_uri_prefix(book_name, path))
}

fn part_uri_prefix(book_name: &str, path: &str) -> String {
    format!(
        "book://{}/{}/",
        percent_encode(book_name),
        percent_encode(path)
    )
}

fn percent_encode(value: &str) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create() -> Self {
            loop {
                let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "memvid-mcp-books-test-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("creating test directory {}: {error}", path.display()),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn book_document(name: &str, title: &str, heading: &str) -> String {
        format!(
            "---\nname: {name}\ntitle: {title}\ndescription: Test Book\nauthor: Tester\n---\n\n# {heading}\n"
        )
    }

    #[test]
    fn splits_utf8_without_losing_content() {
        let content = "中".repeat(MAX_PART_CHARS + 3);
        let parts = split_text(&content);

        assert_eq!(parts.len(), 2, "long UTF-8 text should be split");
        assert_eq!(parts.concat(), content, "split parts must round-trip");
    }

    #[test]
    fn encodes_resource_paths_as_stable_uris() {
        assert_eq!(
            part_uri("Book name", "chapters/a b.md", 7),
            "book://Book%20name/chapters%2Fa%20b.md/part-00007"
        );
    }

    #[test]
    fn creates_updates_and_deletes_book_files() {
        let root = TestDirectory::create();
        let initial = book_document("demo-book", "Demo", "Initial");
        let directory =
            create_book(root.path(), "demo-book", &initial).expect("valid Book should be created");
        let chapter_directory = directory.join("chapters");
        fs::create_dir(&chapter_directory).expect("chapter directory should be created");
        let chapter = chapter_directory.join("one.md");
        fs::write(&chapter, "# Chapter One\n").expect("chapter should be written");
        fs::write(directory.join("cover.png"), b"not markdown")
            .expect("non-Markdown resource should be written");

        let updated = book_document("demo-book", "Updated Demo", "Updated");
        update_book(root.path(), "demo-book", &updated).expect("existing Book should be updated");
        assert_eq!(
            fs::read_to_string(directory.join(BOOK_ENTRY_FILE))
                .expect("updated entry should be readable"),
            updated,
            "update should replace the complete entry"
        );
        assert!(chapter.exists(), "update should preserve chapters");

        let catalog =
            BookCatalog::scan(root.path(), 1024 * 1024).expect("Book directory should scan");
        let book = catalog.book("demo-book").expect("Book should be present");
        assert_eq!(book.title(), "Updated Demo");
        assert_eq!(book.description(), Some("Test Book"));
        assert_eq!(book.author(), Some("Tester"));
        assert_eq!(
            book.file_paths().collect::<Vec<_>>(),
            vec!["BOOK.md", "chapters/one.md"],
            "only Markdown files should be collected"
        );

        delete_book(root.path(), "demo-book").expect("existing Book should be deleted");
        assert!(!directory.exists(), "delete should remove the entire Book");
    }

    #[test]
    fn rejects_unsafe_book_names() {
        let root = TestDirectory::create();
        let error = create_book(
            root.path(),
            "../escape",
            &book_document("../escape", "Unsafe", "Unsafe"),
        )
        .expect_err("path traversal name should be rejected");

        assert!(error.is_request_error(), "unexpected error: {error}");
        assert!(
            error.to_string().contains("single hyphens"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_mismatched_frontmatter_name() {
        let root = TestDirectory::create();
        let error = create_book(
            root.path(),
            "requested-name",
            &book_document("different-name", "Mismatch", "Mismatch"),
        )
        .expect_err("mismatched frontmatter name should be rejected");

        assert!(error.is_request_error(), "unexpected error: {error}");
        assert!(
            error.to_string().contains("must match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn validates_safe_document_file_names_and_extensions() {
        assert!(
            validate_document_file_name("quarterly-report.XLSX").is_ok(),
            "supported extension should be case-insensitive"
        );
        for unsafe_name in [
            "../report.pdf",
            "folder/report.pdf",
            "folder\\report.pdf",
            "report.txt",
            "report.pdf.",
            "bad:name.pdf",
        ] {
            let error = validate_document_file_name(unsafe_name)
                .expect_err("unsafe or unsupported file name should be rejected");
            assert!(error.is_request_error(), "name={unsafe_name}: {error}");
        }
    }

    #[test]
    fn routes_every_supported_extension_to_a_memvid_reader() {
        let registry = ReaderRegistry::default();
        for file_name in [
            "book.pdf",
            "book.docx",
            "slides.pptx",
            "legacy.xls",
            "workbook.xlsx",
        ] {
            let kind = validated_document_kind(file_name)
                .expect("supported extension should identify a document kind");
            let hint =
                ReaderHint::new(Some(kind.mime()), Some(kind.format())).with_uri(Some(file_name));
            assert!(
                registry.find_reader(&hint).is_some(),
                "no Memvid reader registered for {file_name}"
            );
        }
    }

    #[test]
    fn scans_supported_documents_only_from_sources() {
        let root = TestDirectory::create();
        let directory = create_book(
            root.path(),
            "document-book",
            &book_document("document-book", "Documents", "Entry"),
        )
        .expect("Book should be created");
        let sources = directory.join(BOOK_SOURCES_DIR);
        fs::create_dir(&sources).expect("sources directory should be created");
        fs::write(sources.join("report.PDF"), b"fake pdf")
            .expect("supported source should be written");
        fs::write(sources.join("notes.txt"), b"ignored")
            .expect("unsupported source should be written");
        fs::write(directory.join("outside.docx"), b"ignored")
            .expect("document outside sources should be written");
        fs::write(sources.join("hidden.md"), b"# Not a chapter")
            .expect("Markdown in sources should be written");

        let catalog = BookCatalog::scan(root.path(), 1024).expect("Book should scan");
        let book = catalog.book("document-book").expect("Book should exist");
        assert_eq!(
            book.document_paths().collect::<Vec<_>>(),
            vec!["sources/report.PDF"],
            "only supported direct source documents should be listed"
        );
        assert_eq!(
            book.file_paths().collect::<Vec<_>>(),
            vec!["BOOK.md"],
            "sources must not contribute Markdown resources"
        );
    }

    #[test]
    fn stores_document_bytes_in_book_sources() {
        let root = TestDirectory::create();
        create_book(
            root.path(),
            "stored-book",
            &book_document("stored-book", "Stored", "Entry"),
        )
        .expect("Book should be created");
        let bytes = b"minimal fake office bytes";

        let path = store_document(
            root.path(),
            "stored-book",
            "manual.docx",
            bytes,
            bytes.len(),
        )
        .expect("document should be stored without requiring reader success");

        assert_eq!(
            path,
            root.path()
                .join("stored-book")
                .join(BOOK_SOURCES_DIR)
                .join("manual.docx")
        );
        assert_eq!(
            fs::read(path).expect("stored document should be readable"),
            bytes
        );
    }

    #[test]
    fn reads_local_documents_only_from_canonical_allowed_roots() {
        let allowed = TestDirectory::create();
        let outside = TestDirectory::create();
        let source = allowed.path().join("allowed.pdf");
        let outside_source = outside.path().join("outside.pdf");
        fs::write(&source, b"allowed bytes").expect("allowed source should be written");
        fs::write(&outside_source, b"outside bytes").expect("outside source should be written");
        let roots = vec![allowed.path().to_path_buf()];

        let document = read_local_document(&source, &roots, 1024)
            .expect("source below an allowed root should be read");
        assert_eq!(document.file_name, "allowed.pdf");
        assert_eq!(document.bytes, b"allowed bytes");

        let error = read_local_document(&outside_source, &roots, 1024)
            .expect_err("source outside allowed roots should be rejected");
        assert!(error.is_request_error(), "unexpected error: {error}");
        assert!(
            error.to_string().contains("outside allowed roots"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn deserializes_catalogs_without_document_entries() {
        let stored: StoredCatalog = serde_json::from_str(
            r#"{"version":1,"books":{"demo":{"title":"Demo","description":null,"author":null,"entry_file":"BOOK.md","files":{"BOOK.md":1}}}}"#,
        )
        .expect("legacy catalog should deserialize");

        assert!(
            stored.books["demo"].file_blake3.is_empty(),
            "legacy books should default to no Markdown hashes"
        );
        assert!(
            stored.books["demo"].documents.is_empty(),
            "legacy books should default to no documents"
        );
    }

    #[test]
    fn scans_an_empty_book_directory() {
        let root = TestDirectory::create();
        let catalog =
            BookCatalog::scan(root.path(), 1024 * 1024).expect("empty directory should be valid");

        assert_eq!(catalog.iter().count(), 0, "catalog should be empty");
    }
}
