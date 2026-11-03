//! Book 目录扫描、Memvid 同步与精确 Markdown 资源读取。

use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use eyre::Context;
use memvid_core::{DocMetadata, FrameStatus, Memvid, PutOptions};
use serde::{Deserialize, Serialize};

const CATALOG_URI: &str = "books://catalog";
const BOOK_TRACK: &str = "books";
const BOOK_ENTRY_FILE: &str = "BOOK.md";
const MAX_PART_CHARS: usize = 1_800;
const MAX_BOOK_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

/// 当前 Book 目录的内存目录。
#[derive(Clone, Debug)]
pub struct BookCatalog {
    root: PathBuf,
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
}

#[derive(Clone, Debug)]
struct BookFile {
    content: String,
    part_count: usize,
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
    /// 当前可用的 Book 数量。
    pub book_count: usize,
}

/// 创建、更新或删除磁盘 Book 时发生的错误。
#[derive(Debug)]
pub struct BookMutationError {
    kind: BookMutationErrorKind,
    message: String,
    source: Option<io::Error>,
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
            source: Some(source),
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
        self.source.as_ref().map(|source| source as _)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct StoredCatalog {
    version: u32,
    books: BTreeMap<String, StoredBook>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredBook {
    title: String,
    description: Option<String>,
    author: Option<String>,
    entry_file: String,
    files: BTreeMap<String, usize>,
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
    pub fn scan(root: impl AsRef<Path>) -> eyre::Result<Self> {
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

            let definition = scan_book_directory(&directory.path())?;
            let name = definition.name.clone();
            if books.insert(name.clone(), definition).is_some() {
                eyre::bail!("duplicate book name: {name}");
            }
        }

        Ok(Self { root, books })
    }

    /// 返回 Book 根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
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
/// 所有资源使用 `book://` URI，目录使用 `books://catalog` URI。发生变化时，本函数在完成
/// 全部增量写入和删除后只提交一次。
///
/// # Errors
///
/// Memvid 帧读取、写入、删除、序列化或提交失败时返回错误。
pub fn sync_to_memvid(memvid: &mut Memvid, catalog: &BookCatalog) -> eyre::Result<BookSyncReport> {
    let previous = read_stored_catalog(memvid)?;
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
            for (index, part) in split_text(&file.content).iter().enumerate() {
                let uri = part_uri(&book.name, path, index);
                let options = book_part_options(book, path, part);
                if upsert_frame(memvid, &uri, part.as_bytes(), options, metadata_changed)? {
                    report.written_parts += 1;
                } else {
                    report.unchanged_parts += 1;
                }
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
    let catalog_changed = upsert_frame(memvid, CATALOG_URI, &serialized, catalog_options, false)?;

    if report.written_parts > 0 || report.deleted_parts > 0 || catalog_changed {
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

fn scan_book_directory(directory: &Path) -> eyre::Result<BookDefinition> {
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

    Ok(BookDefinition {
        name: directory_name.to_owned(),
        title: metadata.title,
        description: metadata.description,
        author: metadata.author,
        entry_file: BOOK_ENTRY_FILE.to_owned(),
        files,
    })
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

fn upsert_frame(
    memvid: &mut Memvid,
    uri: &str,
    bytes: &[u8],
    mut options: PutOptions,
    force: bool,
) -> eyre::Result<bool> {
    options.uri = Some(uri.to_owned());
    if let Ok(existing) = memvid.frame_by_uri(uri)
        && existing.status == FrameStatus::Active
    {
        let current = memvid.frame_canonical_payload(existing.id)?;
        if !force && current == bytes {
            return Ok(false);
        }
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
                (
                    name.clone(),
                    StoredBook {
                        title: book.title.clone(),
                        description: book.description.clone(),
                        author: book.author.clone(),
                        entry_file: book.entry_file.clone(),
                        files,
                    },
                )
            })
            .collect();
        Self { version: 1, books }
    }
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

        let catalog = BookCatalog::scan(root.path()).expect("Book directory should scan");
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
    fn scans_an_empty_book_directory() {
        let root = TestDirectory::create();
        let catalog = BookCatalog::scan(root.path()).expect("empty directory should be valid");

        assert_eq!(catalog.iter().count(), 0, "catalog should be empty");
    }
}
