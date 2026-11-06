//! Amemory MCP tools for Skill, Book, Document Import, Memory Chunk, and Unified Search operations.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use adapter::{AdapterError, SearchStore};
use domain::DomainError;
use domain::book::{
    BookCatalog, BookFileInput, BookFileUpdatePolicy, CreateBookInput, ImportBookDocumentInput,
    UpdateBookInput,
};
use domain::memory::{
    DEFAULT_MEMORY_NAMESPACE, IndexTarget, MemoryChunkInput, SearchMode, SearchRequest,
};
use domain::skill::{
    CreateSkillInput, SkillCatalog, SkillFileInput, SkillFileUpdatePolicy, UpdateSkillInput,
};
use rmcp::ErrorData as McpError;
use rmcp::handler::server::tool::{ToolCallContext, ToolRouter};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock, Tool};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Document import security limits passed to [`AmemoryToolProvider`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentImportPolicy {
    /// Allowed local filesystem roots for `import_book_document(file_path=...)`.
    pub allowed_roots: Vec<PathBuf>,
    /// Maximum allowed document size in bytes.
    pub max_file_bytes: usize,
}

impl Default for DocumentImportPolicy {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            max_file_bytes: 50 * 1024 * 1024,
        }
    }
}

/// Request parameters for `find_skills`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindSkillsRequest {
    /// Natural-language description of the current task or keywords to match Skills.
    pub query: String,
    /// Maximum number of matching Skills to return (`1..=50`, default `5`).
    #[serde(default = "default_match_limit")]
    pub limit: usize,
}

/// Request parameters for `load_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadSkillRequest {
    /// Target Skill name (`name` from `list_skills` / `find_skills`).
    #[serde(alias = "skill_name")]
    pub name: String,
    /// Optional relative resource file path (e.g. `references/guide.md`). Defaults to `SKILL.md`.
    #[serde(default, alias = "relative_path")]
    pub resource: Option<String>,
}

/// Resource file item for `add_skill` / `update_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillFileItem {
    /// Relative path inside the Skill directory (`scripts/...`, `references/...`, `assets/...`, `agents/...`).
    #[serde(alias = "path")]
    pub relative_path: String,
    /// UTF-8 file content.
    pub content: String,
}

/// Request parameters for `add_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddSkillRequest {
    /// Skill name (`1..=64` lowercase alphanumeric or single hyphens).
    #[serde(default)]
    pub name: Option<String>,
    /// Skill description (`<= 1024` chars, no `<` or `>`).
    #[serde(default)]
    pub description: Option<String>,
    /// Markdown body when `skill_markdown` is omitted.
    #[serde(default)]
    pub body: Option<String>,
    /// Complete `SKILL.md` content including YAML frontmatter.
    #[serde(default)]
    pub skill_markdown: Option<String>,
    /// Optional resource files to create alongside `SKILL.md`.
    #[serde(default)]
    pub files: Vec<SkillFileItem>,
}

/// Request parameters for `update_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSkillRequest {
    /// Target Skill name to update.
    pub name: String,
    /// Updated Skill description.
    #[serde(default)]
    pub description: Option<String>,
    /// Updated Markdown body.
    #[serde(default)]
    pub body: Option<String>,
    /// Complete replacement `SKILL.md` content.
    #[serde(default)]
    pub skill_markdown: Option<String>,
    /// Resource files to add or overwrite.
    #[serde(default)]
    pub files: Vec<SkillFileItem>,
    /// Relative paths of resource files to delete.
    #[serde(default)]
    pub remove_files: Vec<String>,
    /// When `true`, removes all existing resource files not listed in `files`.
    #[serde(default)]
    pub replace_all_files: bool,
}

/// Request parameters for `delete_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteSkillRequest {
    /// Target Skill name to permanently delete.
    #[serde(alias = "skill_name")]
    pub name: String,
}

/// Request parameters for `find_books`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindBooksRequest {
    /// Search query to match against Book titles, descriptions, and chapters.
    pub query: String,
    /// Maximum number of matching Books to return (`1..=50`, default `5`).
    #[serde(default = "default_match_limit")]
    pub limit: usize,
}

/// Request parameters for `load_book`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadBookRequest {
    /// Target Book name.
    #[serde(alias = "book_name")]
    pub name: String,
    /// Optional relative file path (e.g. `chapters/ch01.md`). Defaults to `BOOK.md`.
    #[serde(default, alias = "relative_path")]
    pub resource: Option<String>,
}

/// Resource file item for `add_book` / `update_book`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BookFileItem {
    /// Relative path inside the Book directory (`chapters/...`, `notes/...`, `references/...`, `summaries/...`, `assets/...`, `files/...`).
    #[serde(alias = "path")]
    pub relative_path: String,
    /// UTF-8 file content.
    pub content: String,
}

/// Request parameters for `add_book`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddBookRequest {
    /// Book name (`1..=64` lowercase alphanumeric or single hyphens).
    #[serde(default)]
    pub name: Option<String>,
    /// Book description.
    #[serde(default)]
    pub description: Option<String>,
    /// Markdown body when `book_markdown` is omitted.
    #[serde(default)]
    pub body: Option<String>,
    /// Complete `BOOK.md` content including YAML frontmatter.
    #[serde(default)]
    pub book_markdown: Option<String>,
    /// Optional chapter/note/reference files to create alongside `BOOK.md`.
    #[serde(default)]
    pub files: Vec<BookFileItem>,
}

/// Request parameters for `update_book`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateBookRequest {
    /// Target Book name to update.
    pub name: String,
    /// Updated Book description.
    #[serde(default)]
    pub description: Option<String>,
    /// Updated Markdown body.
    #[serde(default)]
    pub body: Option<String>,
    /// Complete replacement `BOOK.md` content.
    #[serde(default)]
    pub book_markdown: Option<String>,
    /// Chapter/note/reference files to add or overwrite.
    #[serde(default)]
    pub files: Vec<BookFileItem>,
    /// Relative paths of resource files to delete.
    #[serde(default)]
    pub remove_files: Vec<String>,
    /// When `true`, removes all existing resource files not listed in `files`.
    #[serde(default)]
    pub replace_all_files: bool,
}

/// Request parameters for `delete_book`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteBookRequest {
    /// Target Book name to permanently delete.
    #[serde(alias = "book_name")]
    pub name: String,
}

/// Request parameters for `import_book_document`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportBookDocumentRequest {
    /// Optional target Book name; derived from the document filename when omitted.
    #[serde(default, alias = "name")]
    pub book_name: Option<String>,
    /// Optional description when creating a new Book.
    #[serde(default)]
    pub description: Option<String>,
    /// Local document path (must reside under `document_import.allowed_roots`). Mutually exclusive with `content_base64`.
    #[serde(default)]
    pub file_path: Option<String>,
    /// Document filename (`*.pdf`, `*.docx`, `*.pptx`, `*.xlsx`, `*.xls`, `*.md`, `*.txt`).
    #[serde(default)]
    pub file_name: Option<String>,
    /// Base64-encoded document bytes. Mutually exclusive with `file_path`.
    #[serde(default)]
    pub content_base64: Option<String>,
    /// Optional extra tags to attach to imported chunks.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether to extract searchable text from the document (`true` by default).
    #[serde(default = "default_true")]
    pub extract_text: bool,
}

/// Single chunk item inside [`AddChunksRequest`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChunkItem {
    /// Optional explicit chunk ID.
    #[serde(default)]
    pub id: Option<String>,
    /// Optional memory namespace (`default` when omitted).
    #[serde(default)]
    pub namespace: Option<String>,
    /// Optional human-readable title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional logical URI (`memory://...`).
    #[serde(default)]
    pub uri: Option<String>,
    /// Searchable text content of the chunk.
    pub text: String,
    /// Optional metadata tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional structured JSON metadata.
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// Request parameters for `add_chunks`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddChunksRequest {
    /// Batch of memory chunks to index.
    pub chunks: Vec<ChunkItem>,
}

/// Search mode selector for the MCP `search` tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpSearchMode {
    /// Reciprocal rank fusion combining lexical and semantic recall.
    #[default]
    Hybrid,
    /// Full-text BM25 lexical search only.
    Lexical,
    /// Semantic vector similarity search only.
    Semantic,
}

impl From<McpSearchMode> for SearchMode {
    fn from(mode: McpSearchMode) -> Self {
        match mode {
            McpSearchMode::Hybrid => Self::Hybrid,
            McpSearchMode::Lexical => Self::Lexical,
            McpSearchMode::Semantic => Self::Semantic,
        }
    }
}

/// Request parameters for `search`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchToolRequest {
    /// Natural-language or keyword search query.
    pub query: String,
    /// Maximum number of results to return (`1..=100`, default `10`).
    #[serde(default = "default_search_limit", alias = "top_k")]
    pub limit: usize,
    /// Maximum characters in each returned snippet (`80..=4000`, default `400`).
    #[serde(default = "default_snippet_chars")]
    pub snippet_chars: usize,
    /// Search mode (`hybrid`, `lexical`, or `semantic`).
    #[serde(default)]
    pub mode: McpSearchMode,
    /// Optional exact URI filter.
    #[serde(default)]
    pub uri: Option<String>,
    /// Optional URI prefix filter (`skill://`, `book://`, `memory://`).
    #[serde(default)]
    pub scope: Option<String>,
    /// Exact URIs to exclude.
    #[serde(default)]
    pub exclude_uris: Vec<String>,
    /// URI prefixes to exclude.
    #[serde(default)]
    pub exclude_scopes: Vec<String>,
    /// Required tags on returned hits.
    #[serde(default, alias = "tags")]
    pub tag_filters: Vec<String>,
}

const fn default_match_limit() -> usize {
    5
}

const fn default_search_limit() -> usize {
    10
}

const fn default_snippet_chars() -> usize {
    400
}

const fn default_true() -> bool {
    true
}

/// Provider for all Amemory-owned MCP tools (`Skill`, `Book`, `Document`, `Memory`, and `Search`).
#[derive(Clone)]
pub struct AmemoryToolProvider<S: SearchStore + 'static> {
    store: Arc<S>,
    skills: SkillCatalog,
    books: BookCatalog,
    import_policy: DocumentImportPolicy,
    tool_router: ToolRouter<Self>,
}

impl<S: SearchStore + 'static> std::fmt::Debug for AmemoryToolProvider<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AmemoryToolProvider")
            .field("skills_root", &self.skills.root_dir())
            .field("books_root", &self.books.root_dir())
            .finish_non_exhaustive()
    }
}

#[tool_router]
impl<S: SearchStore + 'static> AmemoryToolProvider<S> {
    /// Construct a new [`AmemoryToolProvider`] backed by `store`, `skills`, and `books`.
    pub fn new(
        store: Arc<S>,
        skills: SkillCatalog,
        books: BookCatalog,
        import_policy: DocumentImportPolicy,
    ) -> Self {
        Self {
            store,
            skills,
            books,
            import_policy,
            tool_router: Self::tool_router(),
        }
    }

    /// Return all MCP [`Tool`] definitions registered on this Amemory provider.
    #[must_use]
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }

    /// Look up a single Amemory [`Tool`] definition by name.
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router.get(name).cloned()
    }

    /// Return `true` if `name` is handled by this Amemory provider.
    #[must_use]
    pub fn has_tool(&self, name: &str) -> bool {
        self.tool_router.get(name).is_some()
    }

    /// Dispatch a tool call through `rmcp`'s [`ToolRouter`].
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] if parameter deserialization or tool execution fails.
    pub async fn call_tool(
        &self,
        context: ToolCallContext<'_, Self>,
    ) -> Result<CallToolResponse, McpError> {
        self.tool_router.call(context).await
    }

    /// Health check endpoint returning `"pong"`.
    #[tool(description = "健康检查工具，正常时返回 pong。")]
    pub async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    /// List all Skills in the configured Skill catalog.
    #[tool(description = "列出当前目录中全部可用的开发 Skill 及其简介与资源文件列表。")]
    pub async fn list_skills(&self) -> Result<CallToolResult, McpError> {
        let summaries = self.skills.list_summaries().map_err(map_domain_err)?;
        if summaries.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "当前没有任何已注册的 Skill。",
            )]));
        }
        let mut out = String::new();
        for item in summaries {
            let _ = writeln!(
                out,
                "- name={} files={} ({})\n  description={}",
                item.name,
                item.file_count,
                item.files.join(", "),
                item.description
            );
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.trim_end().to_string(),
        )]))
    }

    /// Find matching Skills by task description or keywords.
    #[tool(description = "根据当前任务描述或关键词检索最相关的开发 Skill。")]
    pub async fn find_skills(
        &self,
        Parameters(request): Parameters<FindSkillsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let matches = self
            .skills
            .find_matches(&request.query, request.limit)
            .map_err(map_domain_err)?;
        if matches.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "未找到匹配该查询的 Skill。",
            )]));
        }
        let mut out = String::new();
        for item in matches {
            let _ = writeln!(
                out,
                "- skill={} score={} files=[{}]\n  description={}\n  preview={}",
                item.name,
                item.score,
                item.files.join(", "),
                item.description,
                item.preview
            );
        }
        out.push_str("下一步: 请对选中的 Skill 调用 load_skill 加载完整说明或资源文件。");
        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }

    /// Load a Skill's `SKILL.md` or one of its referenced files.
    #[tool(
        description = "加载指定 Skill 的完整 SKILL.md 或其内部资源文件（如 references/...、scripts/...）。"
    )]
    pub async fn load_skill(
        &self,
        Parameters(request): Parameters<LoadSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resource = request.resource.as_deref().unwrap_or("SKILL.md");
        let file = self
            .skills
            .load_resource(&request.name, resource)
            .map_err(map_domain_err)?;
        let header = format!(
            "# Skill `{}` — `{}`\n\n{}",
            request.name.trim(),
            file.relative_path,
            file.content
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(header)]))
    }

    /// Create a new Skill on disk and synchronize its chunks into the XERJ Skills index.
    #[tool(description = "创建新的 Skill 目录与 SKILL.md，并立即同步到 XERJ 索引。")]
    pub async fn add_skill(
        &self,
        Parameters(request): Parameters<AddSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .skills
            .create_skill(&CreateSkillInput {
                name: request.name,
                description: request.description,
                body: request.body,
                skill_markdown: request.skill_markdown,
                files: request
                    .files
                    .into_iter()
                    .map(|f| SkillFileInput {
                        relative_path: f.relative_path,
                        content: f.content,
                    })
                    .collect(),
            })
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Update an existing Skill on disk and synchronize its chunks into the XERJ Skills index.
    #[tool(description = "更新已存在的 Skill（SKILL.md 或附属资源文件），并立即同步到 XERJ 索引。")]
    pub async fn update_skill(
        &self,
        Parameters(request): Parameters<UpdateSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let policy = if request.replace_all_files {
            SkillFileUpdatePolicy::ReplaceAll
        } else {
            SkillFileUpdatePolicy::Merge
        };
        let report = self
            .skills
            .update_skill(&UpdateSkillInput {
                name: request.name,
                description: request.description,
                body: request.body,
                skill_markdown: request.skill_markdown,
                files: request
                    .files
                    .into_iter()
                    .map(|f| SkillFileInput {
                        relative_path: f.relative_path,
                        content: f.content,
                    })
                    .collect(),
                remove_files: request.remove_files,
                file_policy: policy,
            })
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Delete a Skill directory from disk and remove its chunks from the XERJ Skills index.
    #[tool(description = "删除指定 Skill 目录，并从 XERJ 索引中清理其全部文档分片。")]
    pub async fn delete_skill(
        &self,
        Parameters(request): Parameters<DeleteSkillRequest>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .skills
            .delete_skill(&request.name)
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Rescan the Skill catalog directory and synchronize all Skills into XERJ idempotently.
    #[tool(description = "重新扫描本地 Skill 目录并幂等同步到 XERJ 索引。")]
    pub async fn sync_skills(&self) -> Result<CallToolResult, McpError> {
        let (scanned, docs) = self
            .skills
            .build_all_index_documents()
            .map_err(map_domain_err)?;
        let report = self
            .store
            .sync_catalog(IndexTarget::Skills, scanned, &docs)
            .await
            .map_err(map_adapter_err)?;
        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// List all Books in the configured Book catalog.
    #[tool(description = "列出当前知识库中全部电子书（Book）及其简介与章节/资源列表。")]
    pub async fn list_books(&self) -> Result<CallToolResult, McpError> {
        let summaries = self.books.list_summaries().map_err(map_domain_err)?;
        if summaries.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "当前没有任何已注册的 Book。",
            )]));
        }
        let mut out = String::new();
        for item in summaries {
            let _ = writeln!(
                out,
                "- name={} files={} ({})\n  description={}",
                item.name,
                item.file_count,
                item.files.join(", "),
                item.description
            );
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.trim_end().to_string(),
        )]))
    }

    /// Find matching Books by query keywords.
    #[tool(description = "根据查询关键词检索最相关的电子书（Book）及其内容预览。")]
    pub async fn find_books(
        &self,
        Parameters(request): Parameters<FindBooksRequest>,
    ) -> Result<CallToolResult, McpError> {
        let matches = self
            .books
            .find_matches(&request.query, request.limit)
            .map_err(map_domain_err)?;
        if matches.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "未找到匹配该查询的 Book。",
            )]));
        }
        let mut out = String::new();
        for item in matches {
            let _ = writeln!(
                out,
                "- book={} score={} files=[{}]\n  description={}\n  preview={}",
                item.name,
                item.score,
                item.files.join(", "),
                item.description,
                item.preview
            );
        }
        out.push_str("下一步: 请对选中的 Book 调用 load_book 加载完整 BOOK.md 或章节文件。");
        Ok(CallToolResult::success(vec![ContentBlock::text(out)]))
    }

    /// Load a Book's `BOOK.md` or one of its chapters/notes.
    #[tool(description = "加载指定 Book 的完整 BOOK.md 或指定章节/笔记资源文件。")]
    pub async fn load_book(
        &self,
        Parameters(request): Parameters<LoadBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resource = request.resource.as_deref().unwrap_or("BOOK.md");
        let file = self
            .books
            .load_resource(&request.name, resource)
            .map_err(map_domain_err)?;
        let header = format!(
            "# Book `{}` — `{}`\n\n{}",
            request.name.trim(),
            file.relative_path,
            file.content
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(header)]))
    }

    /// Create a new Book on disk and synchronize its chunks into the XERJ Books index.
    #[tool(description = "创建新的电子书（Book）目录与 BOOK.md，并立即同步到 XERJ 索引。")]
    pub async fn add_book(
        &self,
        Parameters(request): Parameters<AddBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .books
            .create_book(&CreateBookInput {
                name: request.name,
                description: request.description,
                body: request.body,
                book_markdown: request.book_markdown,
                files: request
                    .files
                    .into_iter()
                    .map(|f| BookFileInput {
                        relative_path: f.relative_path,
                        content: f.content,
                    })
                    .collect(),
            })
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Update an existing Book on disk and synchronize its chunks into the XERJ Books index.
    #[tool(description = "更新已存在的电子书（BOOK.md 或章节/笔记文件），并立即同步到 XERJ 索引。")]
    pub async fn update_book(
        &self,
        Parameters(request): Parameters<UpdateBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let policy = if request.replace_all_files {
            BookFileUpdatePolicy::ReplaceAll
        } else {
            BookFileUpdatePolicy::Merge
        };
        let report = self
            .books
            .update_book(&UpdateBookInput {
                name: request.name,
                description: request.description,
                body: request.body,
                book_markdown: request.book_markdown,
                files: request
                    .files
                    .into_iter()
                    .map(|f| BookFileInput {
                        relative_path: f.relative_path,
                        content: f.content,
                    })
                    .collect(),
                remove_files: request.remove_files,
                file_policy: policy,
            })
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Delete a Book directory from disk and remove its chunks from the XERJ Books index.
    #[tool(description = "删除指定电子书（Book）目录，并从 XERJ 索引中清理其全部文档分片。")]
    pub async fn delete_book(
        &self,
        Parameters(request): Parameters<DeleteBookRequest>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .books
            .delete_book(&request.name)
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Import a local or Base64 document (`pdf`, `docx`, `pptx`, `xlsx`, `xls`, `md`, `txt`) into a Book.
    #[tool(
        description = "将本地或 Base64 文档（pdf/docx/pptx/xlsx/xls/md/txt）导入指定电子书，提取正文并写入 XERJ 索引。"
    )]
    pub async fn import_book_document(
        &self,
        Parameters(request): Parameters<ImportBookDocumentRequest>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .books
            .import_document(
                &ImportBookDocumentInput {
                    book_name: request.book_name,
                    description: request.description,
                    file_path: request.file_path,
                    file_name: request.file_name,
                    content_base64: request.content_base64,
                    tags: request.tags,
                    extract_text: request.extract_text,
                },
                &self.import_policy.allowed_roots,
                self.import_policy.max_file_bytes,
            )
            .map_err(map_domain_err)?;

        self.store
            .apply_mutation(&report.index_mutation)
            .await
            .map_err(map_adapter_err)?;

        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Rescan the Book catalog directory and synchronize all Books into XERJ idempotently.
    #[tool(description = "重新扫描本地 Book 目录并幂等同步到 XERJ 索引。")]
    pub async fn sync_books(&self) -> Result<CallToolResult, McpError> {
        let (scanned, docs) = self
            .books
            .build_all_index_documents()
            .map_err(map_domain_err)?;
        let report = self
            .store
            .sync_catalog(IndexTarget::Books, scanned, &docs)
            .await
            .map_err(map_adapter_err)?;
        let json =
            serde_json::to_string_pretty(&report).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Write general memory chunks into the XERJ Memory index (`amemory-memory-v1`).
    #[tool(description = "向 XERJ 通用记忆索引写入一批文本块（chunks）。")]
    pub async fn add_chunks(
        &self,
        Parameters(request): Parameters<AddChunksRequest>,
    ) -> Result<CallToolResult, McpError> {
        let inputs: Vec<MemoryChunkInput> = request
            .chunks
            .into_iter()
            .map(|c| MemoryChunkInput {
                id: c.id,
                namespace: c
                    .namespace
                    .unwrap_or_else(|| DEFAULT_MEMORY_NAMESPACE.to_string()),
                title: c.title,
                uri: c.uri,
                text: c.text,
                tags: c.tags,
                metadata: c.metadata,
            })
            .collect();

        let ids = self
            .store
            .add_memory_chunks(&inputs)
            .await
            .map_err(map_adapter_err)?;

        let summary = serde_json::json!({
            "added_count": ids.len(),
            "doc_ids": ids,
        });
        let text =
            serde_json::to_string_pretty(&summary).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    /// Execute a unified lexical, semantic, or hybrid search across Amemory Skills, Books, and Memory indices.
    #[tool(description = "跨 Skill、Book 与通用 Memory 索引执行全文、语义或混合检索。")]
    pub async fn search(
        &self,
        Parameters(request): Parameters<SearchToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        let domain_req = SearchRequest {
            query: request.query,
            limit: request.limit,
            snippet_chars: request.snippet_chars,
            mode: request.mode.into(),
            uri: request.uri,
            scope: request.scope,
            exclude_uris: request.exclude_uris,
            exclude_scopes: request.exclude_scopes,
            tag_filters: request.tag_filters,
        };

        let hits = self
            .store
            .search(&domain_req)
            .await
            .map_err(map_adapter_err)?;

        if hits.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "未检索到匹配的结果。",
            )]));
        }

        let text = serde_json::to_string_pretty(&hits).map_err(|e| internal_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

#[allow(clippy::needless_pass_by_value)]
fn map_domain_err(err: DomainError) -> McpError {
    match err {
        DomainError::Validation(_) | DomainError::NotFound(_) | DomainError::AlreadyExists(_) => {
            McpError::invalid_params(err.to_string(), None)
        }
        DomainError::Io(_) => McpError::internal_error(err.to_string(), None),
    }
}

fn map_adapter_err(err: AdapterError) -> McpError {
    match err {
        AdapterError::Domain(domain_err) => map_domain_err(domain_err),
        AdapterError::Query(msg) => McpError::invalid_params(msg, None),
        other => McpError::internal_error(other.to_string(), None),
    }
}

fn internal_err(msg: String) -> McpError {
    McpError::internal_error(msg, None)
}
