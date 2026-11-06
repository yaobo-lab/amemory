//! Stdio transport runner for the unified MCP server.

use amemory_adapter::SearchStore;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::McpFacadeError;
use crate::registry::UnifiedMcpServer;

/// Run the [`UnifiedMcpServer`] over standard input/output (`stdio` transport)
/// until EOF is reached.
///
/// # Errors
///
/// Returns [`McpFacadeError::Transport`] if initializing or running the stdio session fails.
pub async fn serve_stdio<S: SearchStore + 'static>(
    server: UnifiedMcpServer<S>,
) -> Result<(), McpFacadeError> {
    let running = server
        .serve(stdio())
        .await
        .map_err(|e| McpFacadeError::Transport(e.to_string()))?;
    running
        .waiting()
        .await
        .map_err(|e| McpFacadeError::Transport(e.to_string()))?;
    Ok(())
}
