//! XERJ MCP tool provider integration over [`xerj_mcp::McpProvider`].

use rmcp::model::{CallToolResult, ContentBlock, JsonObject, Tool};
use serde_json::{Value, json};
use xerj_mcp::McpProvider;

use crate::McpFacadeError;

/// Wraps [`xerj_mcp::McpProvider`] to expose XERJ's native MCP tools (`xerj_*`)
/// inside the unified Amemory MCP server without duplicating tool implementations.
#[derive(Clone)]
pub struct XerjToolProvider {
    provider: McpProvider,
    tools: Vec<Tool>,
}

impl std::fmt::Debug for XerjToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XerjToolProvider")
            .field("base_url", &self.provider.base_url())
            .field("tool_count", &self.tools.len())
            .finish()
    }
}

impl XerjToolProvider {
    /// Construct a new [`XerjToolProvider`] proxying to `base_url` with optional `auth`.
    ///
    /// # Errors
    ///
    /// Returns [`McpFacadeError`] if initializing the HTTP client or deserializing
    /// XERJ tool specifications fails.
    pub fn new(base_url: impl Into<String>, auth: Option<String>) -> Result<Self, McpFacadeError> {
        let provider = McpProvider::new(base_url, auth)
            .map_err(|e| McpFacadeError::ProviderInit(e.to_string()))?;
        let raw_specs = provider.tool_specs();
        let tools: Vec<Tool> = serde_json::from_value(raw_specs)
            .map_err(|e| McpFacadeError::SchemaConversion(e.to_string()))?;

        Ok(Self { provider, tools })
    }

    /// Return the base URL of the target XERJ ES-compatible listener.
    #[must_use]
    pub fn base_url(&self) -> &str {
        self.provider.base_url()
    }

    /// Return all XERJ MCP tool definitions served by this process.
    #[must_use]
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// Look up a single XERJ MCP tool definition by name.
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name.as_ref() == name)
    }

    /// Return `true` if `name` is a registered XERJ tool.
    #[must_use]
    pub fn has_tool(&self, name: &str) -> bool {
        self.get_tool(name).is_some()
    }

    /// Dispatch a tool invocation to [`xerj_mcp::McpProvider`] and convert the response
    /// into an `rmcp` [`CallToolResult`].
    pub async fn call_tool(&self, name: &str, arguments: Option<JsonObject>) -> CallToolResult {
        let args_value = arguments.map_or_else(|| json!({}), Value::Object);
        let raw_result = self.provider.call_tool(name, args_value).await;
        convert_xerj_tool_result(&raw_result)
    }
}

fn convert_xerj_tool_result(raw: &Value) -> CallToolResult {
    let is_error = raw.get("isError").and_then(Value::as_bool).unwrap_or(false);

    let mut blocks = Vec::new();
    if let Some(items) = raw.get("content").and_then(Value::as_array) {
        for item in items {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                blocks.push(ContentBlock::text(text.to_string()));
            } else {
                blocks.push(ContentBlock::text(item.to_string()));
            }
        }
    }

    if blocks.is_empty() {
        blocks.push(ContentBlock::text(raw.to_string()));
    }

    if is_error {
        CallToolResult::error(blocks)
    } else {
        CallToolResult::success(blocks)
    }
}
