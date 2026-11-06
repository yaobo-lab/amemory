//! Unified MCP tool registry merging Amemory and XERJ tool providers.

use std::collections::BTreeSet;

use adapter::SearchStore;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ResultType, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;

use crate::McpFacadeError;
use crate::amemory::AmemoryToolProvider;
use crate::xerj::XerjToolProvider;

/// Verify that two slices of [`Tool`] definitions have no overlapping names.
///
/// # Errors
///
/// Returns [`McpFacadeError::ToolNameCollision`] if any tool name is duplicated.
pub fn verify_no_tool_conflicts(
    amemory_tools: &[Tool],
    xerj_tools: &[Tool],
) -> Result<(), McpFacadeError> {
    let mut seen = BTreeSet::new();
    for tool in amemory_tools {
        if !seen.insert(tool.name.as_ref()) {
            return Err(McpFacadeError::ToolNameCollision(
                tool.name.as_ref().to_string(),
            ));
        }
    }
    for tool in xerj_tools {
        if !seen.insert(tool.name.as_ref()) {
            return Err(McpFacadeError::ToolNameCollision(
                tool.name.as_ref().to_string(),
            ));
        }
    }
    Ok(())
}

/// Unified MCP server handler combining [`AmemoryToolProvider`] and [`XerjToolProvider`].
#[derive(Clone, Debug)]
pub struct UnifiedMcpServer<S: SearchStore + 'static> {
    amemory: AmemoryToolProvider<S>,
    xerj: XerjToolProvider,
    merged_tools: Vec<Tool>,
}

impl<S: SearchStore + 'static> UnifiedMcpServer<S> {
    /// Construct a [`UnifiedMcpServer`] after verifying there are no tool name collisions.
    ///
    /// # Errors
    ///
    /// Returns [`McpFacadeError::ToolNameCollision`] if any Amemory tool name collides
    /// with a XERJ tool name.
    pub fn new(
        amemory: AmemoryToolProvider<S>,
        xerj: XerjToolProvider,
    ) -> Result<Self, McpFacadeError> {
        let amemory_tools = amemory.list_tools();
        let xerj_tools = xerj.tools();
        verify_no_tool_conflicts(&amemory_tools, xerj_tools)?;

        let mut merged_tools = Vec::with_capacity(amemory_tools.len() + xerj_tools.len());
        merged_tools.extend(amemory_tools);
        merged_tools.extend(xerj_tools.iter().cloned());

        Ok(Self {
            amemory,
            xerj,
            merged_tools,
        })
    }

    /// Return all merged [`Tool`] definitions served by this unified MCP facade.
    #[must_use]
    pub fn list_all_tools(&self) -> &[Tool] {
        &self.merged_tools
    }

    /// Return a reference to the underlying [`AmemoryToolProvider`].
    #[must_use]
    pub const fn amemory_provider(&self) -> &AmemoryToolProvider<S> {
        &self.amemory
    }

    /// Return a reference to the underlying [`XerjToolProvider`].
    #[must_use]
    pub const fn xerj_provider(&self) -> &XerjToolProvider {
        &self.xerj
    }
}

impl<S: SearchStore + 'static> ServerHandler for UnifiedMcpServer<S> {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        let mut server_info = Implementation::new("amemory-mcp", env!("CARGO_PKG_VERSION"));
        server_info.title = Some("Amemory + XERJ Unified MCP Server".to_string());
        server_info.website_url = Some("https://github.com/xerj-org/xerj".to_string());

        ServerConfig::new(capabilities)
            .with_protocol_version(ProtocolVersion::LATEST_WITH_INITIALIZE)
            .with_server_info(server_info)
            .with_instructions(
                "统一 Amemory + XERJ 模型上下文协议 (MCP) 服务：\n\
                 1. Skill 目录与知识调用：在实施、重构、审查或调试前使用 find_skills 检索并使用 load_skill 加载完整 SKILL.md 及引用资源；支持 add_skill、update_skill、delete_skill、sync_skills。\n\
                 2. Book 知识书籍与文档导入：使用 list_books、find_books、load_book 查阅书籍与章节；支持 add_book、update_book、delete_book、import_book_document、sync_books。\n\
                 3. 通用记忆与统一检索：使用 add_chunks 写入记忆片段，使用 search 跨 Skill、Book 与 Memory 索引进行全文/语义/混合检索。\n\
                 4. XERJ 原生搜索引擎与第二大脑工具：完整集成 xerj_search、xerj_semantic_search、xerj_vector_search、xerj_hybrid_search、xerj_memory_store、xerj_memory_recall、xerj_brain_*、xerj_def、xerj_code_search、xerj_map、xerj_status 等原生工具。",
            )
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let supports_cache_hints = context
            .protocol_version()
            .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(ResultType::COMPLETE),
            tools: self.merged_tools.clone(),
            meta: None,
            next_cursor: None,
            ttl_ms: supports_cache_hints.then_some(0),
            cache_scope: supports_cache_hints.then_some(CacheScope::Public),
        })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.merged_tools
            .iter()
            .find(|t| t.name.as_ref() == name)
            .cloned()
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let name = request.name.as_ref();
        if self.amemory.has_tool(name) {
            let tcc = ToolCallContext::new(&self.amemory, request, context);
            self.amemory.call_tool(tcc).await
        } else if self.xerj.has_tool(name) {
            let result = self.xerj.call_tool(name, request.arguments).await;
            Ok(result.into())
        } else {
            Err(McpError::invalid_params(
                format!("未知的 MCP 工具: `{name}`"),
                None,
            ))
        }
    }
}
