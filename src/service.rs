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
    #[tool(description = "健康检查工具：返回“pong”，用于验证 MCP 服务器正在运行且能够正常响应。")]
    pub async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    /// 列出 Memvid 中可加载的 Skill 及其调用策略。
    ///
    /// # Errors
    ///
    /// Skill 目录锁中毒时进程会终止；其他情况下始终成功。
    #[tool(
        description = "列出存储在 Memvid 中的全部开发 Skill，包括说明、入口文件、资源，以及每个 Skill 是否允许自动选择。"
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
        description = "查找与任务相关的开发 Skill。在实施、重构、审查、调试或编写文档前调用此工具，然后为每个相关结果调用 load_skill。结果会排除仅限手动调用的 Skill；此类 Skill 只能通过明确指定名称来加载。"
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
        description = "从 Memvid 加载指定 Skill 的完整说明或引用资源。省略 resource 时加载入口文件。处理当前任务时必须遵循已加载的说明，并按需加载其中引用的文件。"
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
        description = "在配置的 Skill 目录下写入完整的 UTF-8 SKILL.md，以创建新 Skill，然后重新扫描并同步到 Memvid。名称只能包含小写字母、数字和单个连字符，并且必须与 frontmatter.name 一致。如果 Skill 已存在，则操作失败。"
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
        description = "完整替换现有 Skill 的 UTF-8 SKILL.md，同时保留其他资源文件，然后重新扫描并将变更同步到 Memvid。请求的名称必须与 frontmatter.name 一致。"
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
        description = "永久删除现有 Skill 目录，包括 SKILL.md 和全部配套资源；随后重新扫描配置目录，并从 Memvid 中移除对应帧。"
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
        description = "重新扫描配置的 Skill 目录，并将新增、变更和删除的 UTF-8 Skill 资源增量同步到 Memvid。"
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
        description = "列出配置的书籍目录中存储的全部 Markdown 电子书，包括稳定名称、标题、简介、作者、入口文件和 Markdown 资源。"
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
        description = "通过匹配书籍名称、标题、简介和作者，查找与查询相关的 Markdown 电子书。返回元数据和简短的 Memvid 内容片段；调用 load_book 可读取选中的书籍或章节。"
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
        description = "从 Memvid 加载指定电子书的完整 BOOK.md 或某个引用的 Markdown 资源。省略 resource 时加载 BOOK.md；如需加载章节，请使用 list_books 返回的路径。"
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
        description = "在配置的书籍目录下写入完整的 UTF-8 BOOK.md，以创建 Markdown 电子书，然后重新扫描并同步到独立的 Memvid 书籍轨道。如果书籍已存在，则操作失败。"
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
        description = "完整替换现有电子书的 UTF-8 BOOK.md，同时保留章节 Markdown 文件，然后重新扫描并将变更同步到 Memvid。"
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
        description = "永久删除一个电子书目录，包括 BOOK.md、全部章节和配套文件，然后从 Memvid 中移除对应的 book:// 帧。"
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
        description = "将一个 PDF、DOCX、PPTX、XLS 或 XLSX 文档导入现有电子书。book_name 为必填项。如果用户没有明确指定目标书籍，请先调用 list_books，并询问用户应将文档上传到哪本书；不得猜测或自动选择。来源可以是 document_import.allowed_roots 下的 local_path，也可以是 Base64 内容。服务器会将文档保存到 books/<book_name>/sources，运行 Memvid 阅读器，提取并增强文本、执行分块、按配置选择性生成本地 ONNX 或 OpenAI 嵌入、写入 Smart Frames，并更新索引。"
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
        description = "重新扫描配置的书籍目录，并将新增、变更和删除的 UTF-8 Markdown 资源，以及每本书 sources 目录下受支持的文档，增量同步到独立的 Memvid 书籍轨道。"
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
        description = "将文本块作为记忆帧存储到 Memvid。每个文本块都会生成语义嵌入，并以崩溃安全的仅追加格式写入 .mv2 文件。返回已插入的文本块数量及其序列 ID，以供后续引用。"
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
    #[tool(description = "使用自然语言查询检索 Memvid 中存储的通用记忆文本块。")]
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
                        这是 Memvid 记忆、开发 Skill 目录和个人 Markdown 电子书库的模型上下文协议接口。在实施、重构、审查、调试、架构、UI 或文档工作前，使用当前任务调用 find_skills。修改代码前，必须通过 load_skill 加载每个相关结果。当用户询问已存储的书籍时，使用 list_books、find_books 和 load_book。调用 import_book_document 前，必须让用户明确选择目标书籍；如果尚未指定，请调用 list_books 并询问用户，不得猜测。当已加载的 Skill 或书籍引用其他资源时，必须明确加载该资源。不得自动选择仅限手动调用的 Skill；只有用户明确指定名称或调用时才能加载。</overview>
                    <what_is_memvid>
                        Memvid 将知识作为压缩 Smart Frames 存储在单个 .mv2 文件中，类似视频存储帧。它提供约 10 倍的存储效率、低于 5 毫秒的访问速度、支持时间回溯且崩溃安全的仅追加存储；数据、嵌入、搜索索引和元数据都包含在单个可移植文件中，无需外部基础设施。
                    </what_is_memvid>
                    <tools>
                        <tool>
                            <name>ping</name>
                            <purpose>执行健康检查</purpose>
                            <returns>返回“pong”</returns>
                        </tool>
                        <tool>
                            <name>list_skills</name>
                            <purpose>列出全部 Skill、调用策略、入口文件和资源</purpose>
                        </tool>
                        <tool>
                            <name>find_skills</name>
                            <purpose>为当前开发任务匹配可自动调用的 Skill</purpose>
                        </tool>
                        <tool>
                            <name>load_skill</name>
                            <purpose>从 Memvid 加载完整的 Skill 说明或引用资源</purpose>
                        </tool>
                        <tool>
                            <name>add_skill</name>
                            <purpose>创建通过校验的 SKILL.md 并同步到 Memvid</purpose>
                        </tool>
                        <tool>
                            <name>update_skill</name>
                            <purpose>替换现有 SKILL.md 并同步变更</purpose>
                        </tool>
                        <tool>
                            <name>delete_skill</name>
                            <purpose>永久删除 Skill 目录并从 Memvid 中移除</purpose>
                        </tool>
                        <tool>
                            <name>sync_skills</name>
                            <purpose>重新扫描配置的 Skill 目录并同步到 Memvid</purpose>
                        </tool>
                        <tool>
                            <name>list_books</name>
                            <purpose>列出全部 Markdown 电子书及其资源</purpose>
                        </tool>
                        <tool>
                            <name>find_books</name>
                            <purpose>根据查询匹配书籍元数据和内容片段</purpose>
                        </tool>
                        <tool>
                            <name>load_book</name>
                            <purpose>从 Memvid 加载 BOOK.md 或章节资源</purpose>
                        </tool>
                        <tool>
                            <name>add_book</name>
                            <purpose>创建通过校验的 BOOK.md 并同步到 Memvid</purpose>
                        </tool>
                        <tool>
                            <name>update_book</name>
                            <purpose>替换 BOOK.md、保留章节并同步变更</purpose>
                        </tool>
                        <tool>
                            <name>delete_book</name>
                            <purpose>永久删除电子书目录并从 Memvid 中移除</purpose>
                        </tool>
                        <tool>
                            <name>import_book_document</name>
                            <purpose>通过 Memvid 导入管线将 PDF 或 Office 内容导入用户选定的电子书</purpose>
                        </tool>
                        <tool>
                            <name>sync_books</name>
                            <purpose>重新扫描配置的书籍目录并同步到 Memvid</purpose>
                        </tool>
                        <tool>
                            <name>add_chunks</name>
                            <purpose>将通用文本块存储为记忆帧</purpose>
                        </tool>
                        <tool>
                            <name>search</name>
                            <purpose>使用自然语言查询通用记忆</purpose>
                        </tool>
                    </tools>
                    <use_cases>Agent 长期记忆、企业知识库、离线 AI 系统、代码库理解、个人知识管理和可审计的 AI 工作流</use_cases>
                    <storage>全部数据存储在 memvid.mv2 中；该文件是包含压缩帧、全文索引、向量索引和时间线元数据的单个可移植文件，便于共享、版本管理和备份。</storage>
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
