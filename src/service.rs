use std::{
    collections::BTreeSet,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

use memvid_core::{Memvid, SearchRequest};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities,
        ServerConfig,
    },
    tool, tool_handler, tool_router,
};
use tokio::task;

use crate::{
    books::{
        BookCatalog, BookDefinition, BookMutationError, BookSyncReport,
        create_book as create_book_file, delete_book as delete_book_directory, inspect_document,
        load_resource as load_book_resource, read_local_document as read_book_local_document,
        store_document as store_book_document,
        sync_to_memvid_incremental as sync_books_to_memvid_incremental,
        update_book as update_book_file,
    },
    config::DocumentImportConfig,
    embedding::SharedEmbedder,
    models::{
        AddBookRequest, AddChunkRequest, AddSkillRequest, BookDocumentSource, DeleteBookRequest,
        DeleteSkillRequest, FindBooksRequest, FindSkillsRequest, ImportBookDocumentRequest,
        LoadBookRequest, LoadSkillRequest, SearchToolRequest, UpdateBookRequest,
        UpdateSkillRequest,
    },
    skills::{
        SkillCatalog, SkillDefinition, SkillMutationError, SyncReport,
        create_skill as create_skill_file, delete_skill as delete_skill_directory, load_resource,
        sync_to_memvid as sync_skills_to_memvid, update_skill as update_skill_file,
    },
};

/// 处理由共享 Memvid 内存存储支持的 MCP 工具调用。
#[derive(Clone)]
pub struct MemvidService {
    memvid: Arc<Mutex<MemvidSlot>>,
    skills: Arc<RwLock<SkillCatalog>>,
    books: Arc<RwLock<BookCatalog>>,
    document_import: Arc<DocumentImportConfig>,
    embedder: Option<SharedEmbedder>,
    tool_router: ToolRouter<Self>,
}

struct MemvidSlot {
    memory: Option<Memvid>,
    path: PathBuf,
}

impl MemvidSlot {
    const fn new(memory: Memvid, path: PathBuf) -> Self {
        Self {
            memory: Some(memory),
            path,
        }
    }

    fn memory_mut(&mut self) -> Result<&mut Memvid, String> {
        self.memory
            .as_mut()
            .ok_or_else(|| "Memvid storage is unavailable after a failed reopen".to_owned())
    }

    fn synchronize<T>(
        &mut self,
        operation: impl FnOnce(&mut Memvid) -> Result<T, String>,
    ) -> Result<T, String> {
        let result = self.memory_mut().and_then(operation);
        let reopen_result = self.reopen();
        match (result, reopen_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(operation_error), Err(reopen_error)) => {
                Err(format!("{operation_error}; {reopen_error}"))
            }
        }
    }

    fn reopen(&mut self) -> Result<(), String> {
        drop(self.memory.take());
        let reopened = Memvid::open(&self.path).map_err(|error| {
            format!("reopening Memvid storage {}: {error}", self.path.display())
        })?;
        self.memory = Some(reopened);
        Ok(())
    }
}

fn internal(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

fn invalid_params(msg: impl Into<String>) -> McpError {
    McpError::invalid_params(msg.into(), None)
}

#[derive(Debug)]
enum CatalogMutationError {
    Request(String),
    Internal(String),
}

impl CatalogMutationError {
    fn into_mcp_error(self) -> McpError {
        match self {
            Self::Request(message) => invalid_params(message),
            Self::Internal(message) => internal(message),
        }
    }
}

impl From<SkillMutationError> for CatalogMutationError {
    fn from(error: SkillMutationError) -> Self {
        if error.is_request_error() {
            Self::Request(error.to_string())
        } else {
            Self::Internal(error.to_string())
        }
    }
}

impl From<BookMutationError> for CatalogMutationError {
    fn from(error: BookMutationError) -> Self {
        if error.is_request_error() {
            Self::Request(error.to_string())
        } else {
            Self::Internal(error.to_string())
        }
    }
}

#[derive(Debug)]
struct SkillMutationOutcome {
    path: PathBuf,
    report: SyncReport,
}

fn rescan_and_sync_skills(
    catalog: &mut SkillCatalog,
    memvid: &Mutex<MemvidSlot>,
) -> Result<SyncReport, String> {
    let updated = SkillCatalog::scan(catalog.root()).map_err(|error| error.to_string())?;
    let report = {
        let mut slot = memvid.lock().expect("Memvid lock poisoned");
        slot.synchronize(|memory| {
            sync_skills_to_memvid(memory, &updated).map_err(|error| error.to_string())
        })?
    };
    *catalog = updated;
    Ok(report)
}

fn mutate_skill_catalog(
    skills: &RwLock<SkillCatalog>,
    memvid: &Mutex<MemvidSlot>,
    mutation: impl FnOnce(&Path) -> Result<PathBuf, SkillMutationError>,
) -> Result<SkillMutationOutcome, CatalogMutationError> {
    let mut catalog = skills.write().expect("Skill catalog lock poisoned");
    let path = mutation(catalog.root())?;
    let report =
        rescan_and_sync_skills(&mut catalog, memvid).map_err(CatalogMutationError::Internal)?;
    drop(catalog);
    Ok(SkillMutationOutcome { path, report })
}

#[derive(Debug)]
struct BookMutationOutcome {
    path: PathBuf,
    report: BookSyncReport,
}

fn rescan_and_sync_books(
    catalog: &mut BookCatalog,
    memvid: &Mutex<MemvidSlot>,
    embedder: Option<&dyn memvid_core::EmbeddingProvider>,
) -> Result<BookSyncReport, String> {
    let updated = BookCatalog::scan(catalog.root(), catalog.max_document_bytes())
        .map_err(|error| error.to_string())?;
    let report = {
        let mut slot = memvid.lock().expect("Memvid lock poisoned");
        slot.synchronize(|memory| {
            sync_books_to_memvid_incremental(memory, catalog, &updated, embedder)
                .map_err(|error| error.to_string())
        })?
    };
    *catalog = updated;
    Ok(report)
}

fn mutate_book_catalog(
    books: &RwLock<BookCatalog>,
    memvid: &Mutex<MemvidSlot>,
    embedder: Option<&dyn memvid_core::EmbeddingProvider>,
    mutation: impl FnOnce(&Path) -> Result<PathBuf, BookMutationError>,
) -> Result<BookMutationOutcome, CatalogMutationError> {
    let mut catalog = books.write().expect("Book catalog lock poisoned");
    let path = mutation(catalog.root())?;
    let report = rescan_and_sync_books(&mut catalog, memvid, embedder)
        .map_err(CatalogMutationError::Internal)?;
    drop(catalog);
    Ok(BookMutationOutcome { path, report })
}

fn load_document_source(
    source: BookDocumentSource,
    config: &DocumentImportConfig,
) -> Result<(String, Vec<u8>), CatalogMutationError> {
    match source {
        BookDocumentSource::LocalPath { path } => {
            let document = read_book_local_document(
                Path::new(&path),
                &config.allowed_roots,
                config.max_file_bytes,
            )?;
            Ok((document.file_name, document.bytes))
        }
        BookDocumentSource::Base64 { file_name, content } => {
            let encoded = if content.starts_with("data:") {
                let (header, payload) = content.split_once(',').ok_or_else(|| {
                    CatalogMutationError::Request("invalid Base64 data URL".to_owned())
                })?;
                if !header.ends_with(";base64") {
                    return Err(CatalogMutationError::Request(
                        "document data URL must use Base64 encoding".to_owned(),
                    ));
                }
                payload
            } else {
                content.as_str()
            };
            let max_encoded_len = config
                .max_file_bytes
                .saturating_add(2)
                .saturating_div(3)
                .saturating_mul(4);
            if encoded.len() > max_encoded_len {
                return Err(CatalogMutationError::Request(format!(
                    "Base64 document exceeds the {}-byte decoded limit",
                    config.max_file_bytes
                )));
            }
            let bytes = BASE64_STANDARD.decode(encoded).map_err(|error| {
                CatalogMutationError::Request(format!("invalid Base64 document: {error}"))
            })?;
            Ok((file_name, bytes))
        }
    }
}

fn validate_chunks(chunks: &[String]) -> Result<(), String> {
    if chunks.is_empty() {
        return Err("chunks must not be empty".to_owned());
    }

    if let Some(index) = chunks.iter().position(|chunk| chunk.trim().is_empty()) {
        return Err(format!("chunk[{index}] is empty"));
    }

    Ok(())
}

fn validate_search_request(request: &SearchToolRequest) -> Result<(), &'static str> {
    if request.query.trim().is_empty() {
        return Err("query must not be empty");
    }
    if request.top_k == 0 {
        return Err("top_k must be >= 1");
    }

    Ok(())
}

fn validate_find_skills_request(request: &FindSkillsRequest) -> Result<(), &'static str> {
    if request.task.trim().is_empty() {
        return Err("task must not be empty");
    }
    if request.top_k == 0 {
        return Err("top_k must be >= 1");
    }

    Ok(())
}

fn skill_relevance(task: &str, skill: &SkillDefinition) -> usize {
    let task_lower = task.to_lowercase();
    let searchable = format!("{} {}", skill.name(), skill.description()).to_lowercase();
    let mut score = usize::from(task_lower.contains(&skill.name().to_lowercase())) * 100;

    let task_terms = ascii_terms(&task_lower);
    let skill_terms = ascii_terms(&searchable);
    score += task_terms.intersection(&skill_terms).count() * 20;

    let task_bigrams = cjk_bigrams(&task_lower);
    let skill_bigrams = cjk_bigrams(&searchable);
    score += task_bigrams.intersection(&skill_bigrams).count() * 2;
    score
}

fn validate_find_books_request(request: &FindBooksRequest) -> Result<(), &'static str> {
    if request.query.trim().is_empty() {
        return Err("query must not be empty");
    }
    if request.top_k == 0 {
        return Err("top_k must be >= 1");
    }
    Ok(())
}

fn book_relevance(query: &str, book: &BookDefinition) -> usize {
    let query_lower = query.to_lowercase();
    let searchable = format!(
        "{} {} {} {}",
        book.name(),
        book.title(),
        book.description().unwrap_or_default(),
        book.author().unwrap_or_default()
    )
    .to_lowercase();
    let mut score = usize::from(query_lower.contains(&book.name().to_lowercase())) * 100;
    score += usize::from(query_lower.contains(&book.title().to_lowercase())) * 80;

    let query_terms = ascii_terms(&query_lower);
    let book_terms = ascii_terms(&searchable);
    score += query_terms.intersection(&book_terms).count() * 20;

    let query_bigrams = cjk_bigrams(&query_lower);
    let book_bigrams = cjk_bigrams(&searchable);
    score += query_bigrams.intersection(&book_bigrams).count() * 2;
    score
}

fn ascii_terms(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 2)
        .map(str::to_owned)
        .collect()
}

fn cjk_bigrams(text: &str) -> BTreeSet<String> {
    let mut bigrams = BTreeSet::new();
    let mut previous = None;
    for character in text.chars() {
        if is_cjk(character) {
            if let Some(first) = previous {
                bigrams.insert(format!("{first}{character}"));
            }
            previous = Some(character);
        } else {
            previous = None;
        }
    }
    bigrams
}

const fn is_cjk(character: char) -> bool {
    matches!(
        character,
        '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'
    )
}

#[tool_router]
impl MemvidService {
    /// 创建拥有指定内存存储、Skill 目录和电子书目录的服务。
    #[must_use]
    pub fn new(
        memvid: Memvid,
        memory_path: PathBuf,
        skills: SkillCatalog,
        books: BookCatalog,
        document_import: DocumentImportConfig,
        embedder: Option<SharedEmbedder>,
    ) -> Self {
        Self {
            memvid: Arc::new(Mutex::new(MemvidSlot::new(memvid, memory_path))),
            skills: Arc::new(RwLock::new(skills)),
            books: Arc::new(RwLock::new(books)),
            document_import: Arc::new(document_import),
            embedder,
            tool_router: Self::tool_router(),
        }
    }

    /// 验证 MCP 服务器能否接收并响应工具调用。
    ///
    /// # Errors
    ///
    /// 当前实现始终成功，不会返回错误。
    #[tool(
        description = "Health check tool - returns 'pong' to verify the MCP server is running and responsive"
    )]
    pub async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    /// 列出 Memvid 中可加载的 Skill 及其调用策略。
    ///
    /// # Errors
    ///
    /// Skill 目录锁中毒时进程会终止；其他情况下始终成功。
    #[tool(
        description = "List all development Skills stored in Memvid, including descriptions, entry files, resources, and whether each Skill may be selected automatically."
    )]
    pub async fn list_skills(&self) -> Result<CallToolResult, McpError> {
        let catalog = self.skills.read().expect("Skill catalog lock poisoned");
        let mut output = format!(
            "{} Skills are available from {}:\n",
            catalog.iter().count(),
            catalog.root().display()
        );
        for skill in catalog.iter() {
            let invocation = if skill.automatic() {
                "automatic"
            } else {
                "manual-only"
            };
            writeln!(
                output,
                "- {} [{}]\n  description: {}\n  entry: {}\n  resources: {}",
                skill.name(),
                invocation,
                skill.description(),
                skill.entry_file(),
                skill.file_paths().collect::<Vec<_>>().join(", ")
            )
            .expect("writing Skill list to a String cannot fail");
        }
        drop(catalog);

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 根据当前任务从 Memvid 中匹配允许自动调用的 Skill。
    ///
    /// # Errors
    ///
    /// 任务为空或结果上限为零时返回参数无效错误；阻塞任务或 Memvid 检索失败时返回内部错误。
    #[tool(
        description = "Find development Skills relevant to a task. Call this before implementation, refactoring, review, debugging, or documentation work, then call load_skill for every relevant result. Manual-only Skills are intentionally excluded and can only be loaded by explicit name."
    )]
    pub async fn find_skills(
        &self,
        Parameters(request): Parameters<FindSkillsRequest>,
    ) -> Result<CallToolResult, McpError> {
        validate_find_skills_request(&request).map_err(invalid_params)?;
        let result_limit = request.top_k;
        let catalog = self
            .skills
            .read()
            .expect("Skill catalog lock poisoned")
            .clone();
        let memvid = Arc::clone(&self.memvid);
        let output = task::spawn_blocking(move || {
            let mut candidates: Vec<(usize, String)> = catalog
                .iter()
                .filter(|skill| skill.automatic())
                .filter_map(|skill| {
                    let score = skill_relevance(&request.task, skill);
                    (score > 0).then(|| (score, skill.name().to_owned()))
                })
                .collect();
            candidates
                .sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
            candidates.truncate(result_limit);

            if candidates.is_empty() {
                return Ok::<_, String>(
                    "No automatically invocable Skill matched this task.".to_owned(),
                );
            }

            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            let memory = slot.memory_mut()?;
            let mut output = String::new();
            for (score, name) in candidates {
                let skill = catalog
                    .skill(&name)
                    .ok_or_else(|| format!("Skill disappeared from catalog: {name}"))?;
                let (_, content) = load_resource(memory, &catalog, &name, None)
                    .map_err(|error| error.to_string())?;
                let snippet: String = content.chars().take(500).collect();
                writeln!(
                    output,
                    "- skill={} relevance={} entry={}\n  description={}\n  Memvid snippet={}",
                    skill.name(),
                    score,
                    skill.entry_file(),
                    skill.description(),
                    snippet
                )
                .expect("writing Skill matches to a String cannot fail");
            }
            drop(slot);
            output.push_str(
                "Next step: call load_skill for each relevant skill before changing code.",
            );
            Ok(output)
        })
        .await
        .map_err(|error| internal(format!("find_skills worker failed: {error}")))?
        .map_err(internal)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 从 Memvid 精确加载 Skill 主说明或其引用资源。
    ///
    /// # Errors
    ///
    /// Skill/资源不存在、阻塞任务失败或 Memvid 帧无法读取时返回错误。
    #[tool(
        description = "Load the complete instructions or a referenced resource for one named Skill from Memvid. Omit resource to load the entry file. Follow loaded instructions for the current task, and load referenced files on demand."
    )]
    pub async fn load_skill(
        &self,
        Parameters(LoadSkillRequest { name, resource }): Parameters<LoadSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        if name.trim().is_empty() {
            return Err(invalid_params("name must not be empty"));
        }
        if resource.as_ref().is_some_and(|path| path.trim().is_empty()) {
            return Err(invalid_params("resource must not be blank"));
        }

        let catalog = self
            .skills
            .read()
            .expect("Skill catalog lock poisoned")
            .clone();
        let memvid = Arc::clone(&self.memvid);
        let output = task::spawn_blocking(move || {
            let skill = catalog
                .skill(&name)
                .ok_or_else(|| format!("unknown skill: {name}"))?;
            let skill_name = skill.name().to_owned();
            let description = skill.description().to_owned();
            let automatic = skill.automatic();
            let resources = skill.file_paths().collect::<Vec<_>>().join(", ");
            let root = catalog.root().display().to_string();

            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            let memory = slot.memory_mut()?;
            let (path, content) =
                load_resource(memory, &catalog, &skill_name, resource.as_deref())
                    .map_err(|error| error.to_string())?;
            drop(slot);
            let invocation = if automatic {
                "automatic"
            } else {
                "manual-only; load only because the user explicitly selected it"
            };
            Ok::<_, String>(format!(
                "<skill name=\"{skill_name}\" resource=\"{path}\">\nDescription: {description}\nInvocation: {invocation}\nSource root: {root}\nAvailable resources: {resources}\n\n{content}\n</skill>"
            ))
        })
        .await
        .map_err(|error| internal(format!("load_skill worker failed: {error}")))?
        .map_err(invalid_params)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 创建一个新 Skill，并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称或文档无效、Skill 已存在、文件系统写入失败，或同步失败时返回错误。
    #[tool(
        description = "Create a new Skill by writing a complete UTF-8 SKILL.md under the configured Skills directory, then rescan and synchronize it into Memvid. The name must use lowercase letters, digits, and single hyphens, and must match frontmatter.name. Fails if the Skill already exists."
    )]
    pub async fn add_skill(
        &self,
        Parameters(AddSkillRequest { name, content }): Parameters<AddSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let skills = Arc::clone(&self.skills);
        let memvid = Arc::clone(&self.memvid);
        let outcome = task::spawn_blocking(move || {
            mutate_skill_catalog(&skills, &memvid, |root| {
                create_skill_file(root, &name, &content)
            })
        })
        .await
        .map_err(|error| internal(format!("add_skill worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Added Skill at {} and synchronized {} Skills: written_parts={}, unchanged_parts={}, deleted_parts={}",
            outcome.path.display(),
            outcome.report.skill_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 更新现有 Skill 的主说明，并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称或文档无效、Skill 不存在、文件系统写入失败，或同步失败时返回错误。
    #[tool(
        description = "Replace an existing Skill's complete UTF-8 SKILL.md while preserving its other resource files, then rescan and synchronize the change into Memvid. The requested name must match frontmatter.name."
    )]
    pub async fn update_skill(
        &self,
        Parameters(UpdateSkillRequest { name, content }): Parameters<UpdateSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let skills = Arc::clone(&self.skills);
        let memvid = Arc::clone(&self.memvid);
        let outcome = task::spawn_blocking(move || {
            mutate_skill_catalog(&skills, &memvid, |root| {
                update_skill_file(root, &name, &content)
            })
        })
        .await
        .map_err(|error| internal(format!("update_skill worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Updated Skill at {} and synchronized {} Skills: written_parts={}, unchanged_parts={}, deleted_parts={}",
            outcome.path.display(),
            outcome.report.skill_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 删除 Skill 目录及其资源，并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称无效、Skill 不存在、目录删除失败，或同步失败时返回错误。
    #[tool(
        description = "Permanently delete one existing Skill directory, including SKILL.md and all supporting resources, then rescan the configured directory and remove its frames from Memvid."
    )]
    pub async fn delete_skill(
        &self,
        Parameters(DeleteSkillRequest { name }): Parameters<DeleteSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let skills = Arc::clone(&self.skills);
        let memvid = Arc::clone(&self.memvid);
        let outcome = task::spawn_blocking(move || {
            mutate_skill_catalog(&skills, &memvid, |root| delete_skill_directory(root, &name))
        })
        .await
        .map_err(|error| internal(format!("delete_skill worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Deleted Skill at {} and synchronized {} Skills: written_parts={}, unchanged_parts={}, deleted_parts={}",
            outcome.path.display(),
            outcome.report.skill_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 重新扫描磁盘上的 Skill，并增量同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 扫描、阻塞任务、Memvid 写入或提交失败时返回内部错误。
    #[tool(
        description = "Rescan the configured Skills directory and incrementally synchronize added, changed, and deleted UTF-8 Skill resources into Memvid."
    )]
    pub async fn sync_skills(&self) -> Result<CallToolResult, McpError> {
        let skills = Arc::clone(&self.skills);
        let memvid = Arc::clone(&self.memvid);
        let report = task::spawn_blocking(move || {
            let mut catalog = skills.write().expect("Skill catalog lock poisoned");
            rescan_and_sync_skills(&mut catalog, &memvid)
        })
        .await
        .map_err(|error| internal(format!("sync_skills worker failed: {error}")))?
        .map_err(internal)?;

        let output = format!(
            "Synchronized {} Skills: written_parts={}, unchanged_parts={}, deleted_parts={}",
            report.skill_count, report.written_parts, report.unchanged_parts, report.deleted_parts
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 列出全部可加载的 Markdown 电子书。
    ///
    /// # Errors
    ///
    /// Book 目录锁中毒时进程会终止；其他情况下始终成功。
    #[tool(
        description = "List all Markdown electronic books stored in the configured books directory, including stable names, titles, descriptions, authors, entry files, and Markdown resources."
    )]
    pub async fn list_books(&self) -> Result<CallToolResult, McpError> {
        let catalog = self.books.read().expect("Book catalog lock poisoned");
        let mut output = format!(
            "{} Books are available from {}:\n",
            catalog.iter().count(),
            catalog.root().display()
        );
        for book in catalog.iter() {
            writeln!(
                output,
                "- {}\n  title: {}\n  description: {}\n  author: {}\n  entry: {}\n  Markdown resources: {}\n  imported documents: {}",
                book.name(),
                book.title(),
                book.description().unwrap_or("-"),
                book.author().unwrap_or("-"),
                book.entry_file(),
                book.file_paths().collect::<Vec<_>>().join(", "),
                book.document_paths().collect::<Vec<_>>().join(", ")
            )
            .expect("writing Book list to a String cannot fail");
        }
        drop(catalog);

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 根据名称、标题、简介和作者匹配电子书。
    ///
    /// # Errors
    ///
    /// 查询为空、结果上限为零、阻塞任务失败或 Memvid 读取失败时返回错误。
    #[tool(
        description = "Find Markdown electronic books relevant to a query by matching book names, titles, descriptions, and authors. Returns metadata and a short Memvid snippet; call load_book to read the selected book or chapter."
    )]
    pub async fn find_books(
        &self,
        Parameters(request): Parameters<FindBooksRequest>,
    ) -> Result<CallToolResult, McpError> {
        validate_find_books_request(&request).map_err(invalid_params)?;
        let result_limit = request.top_k;
        let catalog = self
            .books
            .read()
            .expect("Book catalog lock poisoned")
            .clone();
        let memvid = Arc::clone(&self.memvid);
        let output = task::spawn_blocking(move || {
            let mut candidates: Vec<(usize, String)> = catalog
                .iter()
                .filter_map(|book| {
                    let score = book_relevance(&request.query, book);
                    (score > 0).then(|| (score, book.name().to_owned()))
                })
                .collect();
            candidates
                .sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
            candidates.truncate(result_limit);

            if candidates.is_empty() {
                return Ok::<_, String>("No Book matched this query.".to_owned());
            }

            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            let memory = slot.memory_mut()?;
            let mut output = String::new();
            for (score, name) in candidates {
                let book = catalog
                    .book(&name)
                    .ok_or_else(|| format!("Book disappeared from catalog: {name}"))?;
                let (_, content) = load_book_resource(memory, &catalog, &name, None)
                    .map_err(|error| error.to_string())?;
                let snippet: String = content.chars().take(500).collect();
                writeln!(
                    output,
                    "- book={} relevance={} title={} author={} entry={}\n  description={}\n  Memvid snippet={}",
                    book.name(),
                    score,
                    book.title(),
                    book.author().unwrap_or("-"),
                    book.entry_file(),
                    book.description().unwrap_or("-"),
                    snippet
                )
                .expect("writing Book matches to a String cannot fail");
            }
            drop(slot);
            output.push_str("Next step: call load_book for the selected book or chapter.");
            Ok(output)
        })
        .await
        .map_err(|error| internal(format!("find_books worker failed: {error}")))?
        .map_err(internal)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 从 Memvid 加载电子书入口或指定 Markdown 资源。
    ///
    /// # Errors
    ///
    /// Book/资源不存在、阻塞任务失败或 Memvid 帧无法读取时返回错误。
    #[tool(
        description = "Load the complete BOOK.md or one referenced Markdown resource for a named electronic book from Memvid. Omit resource to load BOOK.md; use a path returned by list_books to load a chapter."
    )]
    pub async fn load_book(
        &self,
        Parameters(LoadBookRequest { name, resource }): Parameters<LoadBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        if name.trim().is_empty() {
            return Err(invalid_params("name must not be empty"));
        }
        if resource.as_ref().is_some_and(|path| path.trim().is_empty()) {
            return Err(invalid_params("resource must not be blank"));
        }

        let catalog = self
            .books
            .read()
            .expect("Book catalog lock poisoned")
            .clone();
        let memvid = Arc::clone(&self.memvid);
        let output = task::spawn_blocking(move || {
            let book = catalog
                .book(&name)
                .ok_or_else(|| format!("unknown book: {name}"))?;
            let book_name = book.name().to_owned();
            let title = book.title().to_owned();
            let description = book.description().unwrap_or("-").to_owned();
            let author = book.author().unwrap_or("-").to_owned();
            let resources = book.file_paths().collect::<Vec<_>>().join(", ");
            let root = catalog.root().display().to_string();

            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            let memory = slot.memory_mut()?;
            let (path, content) =
                load_book_resource(memory, &catalog, &book_name, resource.as_deref())
                    .map_err(|error| error.to_string())?;
            drop(slot);
            Ok::<_, String>(format!(
                "<book name=\"{book_name}\" resource=\"{path}\">\nTitle: {title}\nDescription: {description}\nAuthor: {author}\nSource root: {root}\nAvailable resources: {resources}\n\n{content}\n</book>"
            ))
        })
        .await
        .map_err(|error| internal(format!("load_book worker failed: {error}")))?
        .map_err(invalid_params)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 创建一本电子书并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称或文档无效、Book 已存在、文件系统写入失败，或同步失败时返回错误。
    #[tool(
        description = "Create a Markdown electronic book by writing a complete UTF-8 BOOK.md under the configured books directory, then rescan and synchronize it into the isolated Memvid books track. Fails if the Book already exists."
    )]
    pub async fn add_book(
        &self,
        Parameters(AddBookRequest { name, content }): Parameters<AddBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let books = Arc::clone(&self.books);
        let memvid = Arc::clone(&self.memvid);
        let embedder = self.embedder.clone();
        let outcome = task::spawn_blocking(move || {
            mutate_book_catalog(&books, &memvid, embedder.as_deref(), |root| {
                create_book_file(root, &name, &content)
            })
        })
        .await
        .map_err(|error| internal(format!("add_book worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Added Book at {} and synchronized {} Books: written_parts={}, unchanged_parts={}, deleted_parts={}, written_documents={}, unchanged_documents={}, deleted_documents={}",
            outcome.path.display(),
            outcome.report.book_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts,
            outcome.report.written_documents,
            outcome.report.unchanged_documents,
            outcome.report.deleted_documents
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 更新现有电子书的入口文件并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称或文档无效、Book 不存在、文件系统写入失败，或同步失败时返回错误。
    #[tool(
        description = "Replace an existing electronic book's complete UTF-8 BOOK.md while preserving chapter Markdown files, then rescan and synchronize the change into Memvid."
    )]
    pub async fn update_book(
        &self,
        Parameters(UpdateBookRequest { name, content }): Parameters<UpdateBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let books = Arc::clone(&self.books);
        let memvid = Arc::clone(&self.memvid);
        let embedder = self.embedder.clone();
        let outcome = task::spawn_blocking(move || {
            mutate_book_catalog(&books, &memvid, embedder.as_deref(), |root| {
                update_book_file(root, &name, &content)
            })
        })
        .await
        .map_err(|error| internal(format!("update_book worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Updated Book at {} and synchronized {} Books: written_parts={}, unchanged_parts={}, deleted_parts={}, written_documents={}, unchanged_documents={}, deleted_documents={}",
            outcome.path.display(),
            outcome.report.book_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts,
            outcome.report.written_documents,
            outcome.report.unchanged_documents,
            outcome.report.deleted_documents
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 删除整本电子书并立即同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 名称无效、Book 不存在、目录删除失败，或同步失败时返回错误。
    #[tool(
        description = "Permanently delete one electronic book directory, including BOOK.md and every chapter or supporting file, then remove its book:// frames from Memvid."
    )]
    pub async fn delete_book(
        &self,
        Parameters(DeleteBookRequest { name }): Parameters<DeleteBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let books = Arc::clone(&self.books);
        let memvid = Arc::clone(&self.memvid);
        let embedder = self.embedder.clone();
        let outcome = task::spawn_blocking(move || {
            mutate_book_catalog(&books, &memvid, embedder.as_deref(), |root| {
                delete_book_directory(root, &name)
            })
        })
        .await
        .map_err(|error| internal(format!("delete_book worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Deleted Book at {} and synchronized {} Books: written_parts={}, unchanged_parts={}, deleted_parts={}, written_documents={}, unchanged_documents={}, deleted_documents={}",
            outcome.path.display(),
            outcome.report.book_count,
            outcome.report.written_parts,
            outcome.report.unchanged_parts,
            outcome.report.deleted_parts,
            outcome.report.written_documents,
            outcome.report.unchanged_documents,
            outcome.report.deleted_documents
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 上传 PDF 或 Office 文档并归档到指定电子书。
    ///
    /// # Errors
    ///
    /// 未指定现有 Book、输入来源不安全、Base64 或文档格式无效、提取/嵌入失败，或者同步失败时返回错误。
    #[tool(
        description = "Import one PDF, DOCX, PPTX, XLS, or XLSX document into an existing Book. book_name is mandatory. If the user has not explicitly named the destination Book, first call list_books and ask the user which Book should receive the upload; never guess or auto-select it. The source may be a local_path under document_import.allowed_roots or Base64 content. The server saves it under books/<book_name>/sources, runs the Memvid reader, extracts and enriches text, chunks it, optionally generates configured local ONNX or OpenAI embeddings, writes Smart Frames, and updates indexes."
    )]
    pub async fn import_book_document(
        &self,
        Parameters(ImportBookDocumentRequest { book_name, source }): Parameters<
            ImportBookDocumentRequest,
        >,
    ) -> Result<CallToolResult, McpError> {
        if book_name.trim().is_empty() {
            return Err(invalid_params("book_name must not be empty"));
        }

        let books = Arc::clone(&self.books);
        let memvid = Arc::clone(&self.memvid);
        let config = Arc::clone(&self.document_import);
        let embedder = self.embedder.clone();
        let embedding_label = embedder.as_deref().map_or_else(
            || "none (lexical index only)".to_owned(),
            |provider| format!("{}:{}", provider.kind(), provider.model()),
        );
        let (outcome, inspection) = task::spawn_blocking(move || {
            let (file_name, bytes) = load_document_source(source, &config)?;
            let inspection = inspect_document(&file_name, &bytes, config.max_file_bytes)?;
            let outcome = mutate_book_catalog(&books, &memvid, embedder.as_deref(), |root| {
                store_book_document(root, &book_name, &file_name, &bytes, config.max_file_bytes)
            })?;
            Ok::<_, CatalogMutationError>((outcome, inspection))
        })
        .await
        .map_err(|error| internal(format!("import_book_document worker failed: {error}")))?
        .map_err(CatalogMutationError::into_mcp_error)?;

        let output = format!(
            "Imported document '{}' ({} bytes, {} extracted chars) into Book at {}. Pipeline completed: reader={}, fallback={}, warnings={}, enrichment, chunking, Smart Frame write, and index update. embedding={}; written_documents={}, unchanged_documents={}, deleted_documents={}",
            inspection.file_name,
            inspection.bytes,
            inspection.extracted_chars,
            outcome.path.display(),
            inspection.reader_name,
            inspection.fallback,
            inspection.warnings.len(),
            embedding_label,
            outcome.report.written_documents,
            outcome.report.unchanged_documents,
            outcome.report.deleted_documents
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 重新扫描磁盘电子书目录并增量同步到 Memvid。
    ///
    /// # Errors
    ///
    /// 扫描、阻塞任务、Memvid 写入或提交失败时返回内部错误。
    #[tool(
        description = "Rescan the configured books directory and incrementally synchronize added, changed, and deleted UTF-8 Markdown resources plus supported documents under each Book's sources directory into the isolated Memvid books track."
    )]
    pub async fn sync_books(&self) -> Result<CallToolResult, McpError> {
        let books = Arc::clone(&self.books);
        let memvid = Arc::clone(&self.memvid);
        let embedder = self.embedder.clone();
        let report = task::spawn_blocking(move || {
            let mut catalog = books.write().expect("Book catalog lock poisoned");
            rescan_and_sync_books(&mut catalog, &memvid, embedder.as_deref())
        })
        .await
        .map_err(|error| internal(format!("sync_books worker failed: {error}")))?
        .map_err(internal)?;

        let output = format!(
            "Synchronized {} Books: written_parts={}, unchanged_parts={}, deleted_parts={}, written_documents={}, unchanged_documents={}, deleted_documents={}",
            report.book_count,
            report.written_parts,
            report.unchanged_parts,
            report.deleted_parts,
            report.written_documents,
            report.unchanged_documents,
            report.deleted_documents
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }

    /// 将验证通过的文本块追加到内存存储中。
    ///
    /// # Errors
    ///
    /// 请求为空或包含空白文本块时返回参数无效错误；阻塞任务、写入或提交失败时返回内部错误。
    #[tool(
        description = "Store text chunks as memory frames in Memvid. Each chunk is encoded with semantic embeddings and appended to the .mv2 file in a crash-safe, append-only format. Returns the number of inserted chunks and their sequence IDs for reference."
    )]
    pub async fn add_chunks(
        &self,
        Parameters(AddChunkRequest { chunks }): Parameters<AddChunkRequest>,
    ) -> Result<CallToolResult, McpError> {
        validate_chunks(&chunks).map_err(invalid_params)?;

        let memvid = Arc::clone(&self.memvid);
        let sequences = task::spawn_blocking(move || {
            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            slot.synchronize(|memory| {
                let mut sequences = Vec::with_capacity(chunks.len());

                for chunk in chunks {
                    let sequence = memory
                        .put_bytes(chunk.as_bytes())
                        .map_err(|error| format!("put_bytes failed: {error}"))?;
                    sequences.push(sequence);
                }

                memory
                    .commit()
                    .map_err(|error| format!("commit failed: {error}"))?;
                Ok(sequences)
            })
        })
        .await
        .map_err(|error| internal(format!("add_chunks worker failed: {error}")))?
        .map_err(internal)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "Inserted {} chunks. Sequences: {:?}",
            sequences.len(),
            sequences
        ))]))
    }

    /// 使用自然语言查询检索已存储的文本块。
    ///
    /// # Errors
    ///
    /// 查询为空或结果数量限制为零时返回参数无效错误；阻塞任务或检索失败时返回内部错误。
    #[tool(description = "Search")]
    pub async fn search(
        &self,
        Parameters(request): Parameters<SearchToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        validate_search_request(&request).map_err(invalid_params)?;

        let memvid = Arc::clone(&self.memvid);
        let response = task::spawn_blocking(move || {
            let search_request = SearchRequest {
                query: request.query,
                top_k: request.top_k,
                snippet_chars: request.snippet_chars,
                uri: None,
                scope: None,
                cursor: None,
                as_of_frame: None,
                as_of_ts: None,
                no_sketch: false,
                acl_context: None,
                acl_enforcement_mode: memvid_core::AclEnforcementMode::default(),
            };

            let mut slot = memvid.lock().expect("Memvid lock poisoned");
            slot.memory_mut()?
                .search(search_request)
                .map_err(|error| format!("search failed: {error}"))
        })
        .await
        .map_err(|error| internal(format!("search worker failed: {error}")))?
        .map_err(internal)?;

        let mut output = format!(
            "total_hits={}, elapsed_ms={}\n",
            response.total_hits, response.elapsed_ms
        );
        for hit in &response.hits {
            let title = hit.title.as_deref().unwrap_or("Untitled");
            let score = hit.score.unwrap_or(0.0);
            writeln!(
                output,
                "- frame_id={}, title={}, score={:.3}\n  snippet={}",
                hit.frame_id, title, score, hit.text
            )
            .expect("writing search results to a String cannot fail");
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "the rmcp tool_handler macro generates the asynchronous handler"
)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for MemvidService {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        let mut server_info = Implementation::new("Memvid MCP", env!("CARGO_PKG_VERSION"));
        server_info.title = Some("Memvid Memory Server".to_string());
        server_info.website_url = Some("https://github.com/memvid/memvid".to_string());

        ServerConfig::new(capabilities)
            // 2026-07-28 协议已取消 initialize；stdio 仍使用传统初始化流程。
            .with_protocol_version(ProtocolVersion::LATEST_WITH_INITIALIZE)
            .with_server_info(server_info)
            .with_instructions(
                r"
                <memvid_mcp_server>
                    <overview>
                        Model Context Protocol interface to Memvid memory, a development Skill catalog, and a personal Markdown electronic-book library. Before implementation, refactoring, review, debugging, architecture, UI, or documentation work, call find_skills with the task. Load every relevant result with load_skill before changing code. Use list_books, find_books, and load_book when the user asks about stored books. Before calling import_book_document, require the user to explicitly choose the destination Book; if it is unspecified, call list_books and ask instead of guessing. When a loaded Skill or Book references another resource, load that resource explicitly. Never auto-select a manual-only Skill; load it only when the user explicitly names or invokes it.</overview>
                    <what_is_memvid>
                        Memvid stores knowledge as compressed Smart Frames in a single .mv2 file (like video stores frames). Provides: 10x storage efficiency, sub-5ms access, append-only crash-safe storage with time-travel, single portable file with data/embeddings/search/metadata, no external infrastructure needed
                    </what_is_memvid>
                    <tools>
                        <tool>
                            <name>ping</name>
                            <purpose>Health check</purpose>
                            <returns>'pong'</returns>
                        </tool>
                        <tool>
                            <name>list_skills</name>
                            <purpose>List all Skills, invocation policies, entry files, and resources</purpose>
                        </tool>
                        <tool>
                            <name>find_skills</name>
                            <purpose>Match the current development task to automatically invocable Skills</purpose>
                        </tool>
                        <tool>
                            <name>load_skill</name>
                            <purpose>Load complete Skill instructions or a referenced resource from Memvid</purpose>
                        </tool>
                        <tool>
                            <name>add_skill</name>
                            <purpose>Create a validated SKILL.md and synchronize it into Memvid</purpose>
                        </tool>
                        <tool>
                            <name>update_skill</name>
                            <purpose>Replace an existing SKILL.md and synchronize the change</purpose>
                        </tool>
                        <tool>
                            <name>delete_skill</name>
                            <purpose>Permanently delete a Skill directory and remove it from Memvid</purpose>
                        </tool>
                        <tool>
                            <name>sync_skills</name>
                            <purpose>Rescan the configured Skill directory and synchronize Memvid</purpose>
                        </tool>
                        <tool>
                            <name>list_books</name>
                            <purpose>List all Markdown electronic books and resources</purpose>
                        </tool>
                        <tool>
                            <name>find_books</name>
                            <purpose>Match a query to book metadata and a content snippet</purpose>
                        </tool>
                        <tool>
                            <name>load_book</name>
                            <purpose>Load BOOK.md or a chapter resource from Memvid</purpose>
                        </tool>
                        <tool>
                            <name>add_book</name>
                            <purpose>Create a validated BOOK.md and synchronize it into Memvid</purpose>
                        </tool>
                        <tool>
                            <name>update_book</name>
                            <purpose>Replace BOOK.md while preserving chapters and synchronize it</purpose>
                        </tool>
                        <tool>
                            <name>delete_book</name>
                            <purpose>Permanently delete a Book directory and remove it from Memvid</purpose>
                        </tool>
                        <tool>
                            <name>import_book_document</name>
                            <purpose>Import PDF or Office content into a user-selected Book through the Memvid ingestion pipeline</purpose>
                        </tool>
                        <tool>
                            <name>sync_books</name>
                            <purpose>Rescan the configured books directory and synchronize Memvid</purpose>
                        </tool>
                        <tool>
                            <name>add_chunks</name>
                            <purpose>Store general text chunks as memory frames</purpose>
                        </tool>
                        <tool>
                            <name>search</name>
                            <purpose>Query general memory using natural language</purpose>
                        </tool>
                    </tools>
                    <use_cases>Agent long-term memory, enterprise knowledge bases, offline AI systems, codebase understanding, personal knowledge, auditable AI workflows</use_cases>
                    <storage>All data in memvid.mv2 - single portable file with compressed frames, full-text index, vector index, timeline metadata. Shareable, versionable, backup-friendly</storage>
                </memvid_mcp_server>",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_chunk_list() {
        let chunks = Vec::new();

        assert_eq!(
            validate_chunks(&chunks),
            Err("chunks must not be empty".to_owned()),
            "an empty request must be rejected before opening the store"
        );
    }

    #[test]
    fn rejects_blank_chunk_before_writing() {
        let chunks = vec!["valid".to_owned(), "  \n".to_owned()];

        assert_eq!(
            validate_chunks(&chunks),
            Err("chunk[1] is empty".to_owned()),
            "all chunks must be validated before the first write"
        );
    }

    #[test]
    fn accepts_non_empty_chunks() {
        let chunks = vec!["first".to_owned(), "second".to_owned()];

        assert!(
            validate_chunks(&chunks).is_ok(),
            "non-empty chunks should pass validation"
        );
    }

    #[test]
    fn decodes_plain_and_data_url_document_uploads() {
        let config = DocumentImportConfig {
            max_file_bytes: 16,
            allowed_roots: Vec::new(),
        };
        for content in ["aGVsbG8=", "data:application/pdf;base64,aGVsbG8="] {
            let (file_name, bytes) = load_document_source(
                BookDocumentSource::Base64 {
                    file_name: "book.pdf".to_owned(),
                    content: content.to_owned(),
                },
                &config,
            )
            .expect("valid Base64 should decode");
            assert_eq!(file_name, "book.pdf");
            assert_eq!(bytes, b"hello");
        }
    }

    #[test]
    fn rejects_oversized_base64_before_decoding() {
        let config = DocumentImportConfig {
            max_file_bytes: 2,
            allowed_roots: Vec::new(),
        };
        let error = load_document_source(
            BookDocumentSource::Base64 {
                file_name: "book.pdf".to_owned(),
                content: "aGVsbG8=".to_owned(),
            },
            &config,
        )
        .expect_err("oversized Base64 should be rejected");

        assert!(
            matches!(error, CatalogMutationError::Request(_)),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn rejects_blank_search_query() {
        let request = SearchToolRequest {
            query: " \t".to_owned(),
            top_k: 1,
            snippet_chars: 100,
        };

        assert_eq!(
            validate_search_request(&request),
            Err("query must not be empty"),
            "blank queries must not reach the store"
        );
    }

    #[test]
    fn rejects_zero_search_limit() {
        let request = SearchToolRequest {
            query: "memory".to_owned(),
            top_k: 0,
            snippet_chars: 100,
        };

        assert_eq!(
            validate_search_request(&request),
            Err("top_k must be >= 1"),
            "a zero result limit is invalid"
        );
    }
}
