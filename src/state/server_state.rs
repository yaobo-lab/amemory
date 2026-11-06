//! Server runtime state and background service metrics.



use crate::core::types::StatsSnapshot;
use crate::services::update::UpdatePhase;

/// Server runtime state observed by UI.
#[derive(Debug, Clone, Default)]
pub struct ServerRuntimeState {
    /// Live statistics snapshot.
    pub stats: StatsSnapshot,
    /// Whether background MCP server is active.
    pub mcp_running: bool,
    /// Current update progress phase.
    pub update_phase: Option<UpdatePhase>,
    /// Whether a catalog sync task is currently in progress.
    pub is_syncing: bool,
}
