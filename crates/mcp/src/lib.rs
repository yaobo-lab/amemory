//! Unified MCP facade combining Amemory's Skill/Book/Memory tools and XERJ's native MCP tools.

pub mod amemory;
pub mod http;
pub mod registry;
pub mod stdio;
pub mod xerj;

use thiserror::Error;

pub use amemory::{AmemoryToolProvider, DocumentImportPolicy};
pub use http::{HttpMcpConfig, build_http_router, serve_http};
pub use registry::{UnifiedMcpServer, verify_no_tool_conflicts};
pub use stdio::serve_stdio;
pub use xerj::XerjToolProvider;
pub use rmcp;

/// Errors returned when constructing or serving the unified MCP facade.
#[derive(Debug, Error)]
pub enum McpFacadeError {
    /// Tool name collision detected between Amemory and XERJ tool sets.
    #[error("检测到 MCP 工具名称冲突: `{0}`")]
    ToolNameCollision(String),

    /// Failed to initialize the underlying XERJ MCP provider.
    #[error("初始化 XERJ MCP Provider 失败: {0}")]
    ProviderInit(String),

    /// Failed to deserialize XERJ tool specifications into `rmcp` tool models.
    #[error("解析 XERJ 工具 Schema 失败: {0}")]
    SchemaConversion(String),

    /// MCP stdio or HTTP transport error.
    #[error("MCP 传输层错误: {0}")]
    Transport(String),
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::sync::Arc;

    use adapter::{XerjAdapter, XerjConfig};
    use domain::book::BookCatalog;
    use domain::skill::{CreateSkillInput, SkillCatalog};
    use rmcp::model::{JsonObject, Tool};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unified_mcp_registry_merges_amemory_and_xerj_tools_without_conflicts() {
        let dir = TempDir::new().expect("tempdir");
        let skills_dir = dir.path().join("skills");
        let books_dir = dir.path().join("books");
        let xerj_dir = dir.path().join("xerj");

        let skills = SkillCatalog::new(&skills_dir);
        skills
            .create_skill(&CreateSkillInput {
                name: Some("test-skill".to_string()),
                description: Some("A test skill for MCP verification".to_string()),
                body: Some("# Test Skill\n\nBody text.".to_string()),
                skill_markdown: None,
                files: Vec::new(),
            })
            .expect("create skill");
        let books = BookCatalog::new(&books_dir);

        let adapter = Arc::new(
            XerjAdapter::open(XerjConfig {
                data_dir: xerj_dir,
                ..XerjConfig::default()
            })
            .expect("open adapter"),
        );
        let loopback = adapter
            .start_loopback_es_listener()
            .await
            .expect("start loopback ES listener");

        let amemory_provider = AmemoryToolProvider::new(
            adapter.clone(),
            skills,
            books,
            DocumentImportPolicy::default(),
        );
        let xerj_provider =
            XerjToolProvider::new(loopback.base_url(), None).expect("create xerj provider");

        let amemory_count = amemory_provider.list_tools().len();
        let xerj_count = xerj_provider.tools().len();
        assert_eq!(amemory_count, 18, "Amemory must expose 18 tools");
        assert!(
            xerj_count == 20 || xerj_count == 23,
            "XERJ must expose 20 (no hub key) or 23 (with hub key) tools, got {xerj_count}"
        );

        let unified =
            UnifiedMcpServer::new(amemory_provider, xerj_provider).expect("build unified server");
        assert_eq!(unified.list_all_tools().len(), amemory_count + xerj_count);

        // Verify a native XERJ tool call (`xerj_status`) works end-to-end against our loopback listener!
        let status_res = unified.xerj_provider().call_tool("xerj_status", None).await;
        assert_ne!(
            status_res.is_error,
            Some(true),
            "xerj_status should succeed"
        );

        // Verify `xerj_memory_store` and `xerj_memory_recall` work end-to-end!
        let store_args: JsonObject = serde_json::from_value(json!({
            "namespace": "test-ns",
            "text": "XERJ is the unified search and memory engine for Amemory."
        }))
        .expect("store args");
        let store_res = unified
            .xerj_provider()
            .call_tool("xerj_memory_store", Some(store_args))
            .await;
        assert_ne!(
            store_res.is_error,
            Some(true),
            "xerj_memory_store failed: {store_res:?}"
        );

        let recall_args: JsonObject = serde_json::from_value(json!({
            "namespace": "test-ns",
            "query": "unified search and memory engine"
        }))
        .expect("recall args");
        let recall_res = unified
            .xerj_provider()
            .call_tool("xerj_memory_recall", Some(recall_args))
            .await;
        assert_ne!(
            recall_res.is_error,
            Some(true),
            "xerj_memory_recall failed: {recall_res:?}"
        );

        loopback.shutdown().await;
    }

    #[test]
    fn duplicate_tool_name_fails_fast() {
        let mut fake = Tool::default();
        fake.name = Cow::Borrowed("xerj_search");
        fake.input_schema = Arc::new(JsonObject::new());

        let err = verify_no_tool_conflicts(&[fake.clone()], &[fake]).unwrap_err();
        assert!(err.to_string().contains("xerj_search"));
    }

    #[test]
    fn xerj_tool_provider_aligns_with_upstream_tool_specs_with_and_without_hub_key() {
        let upstream_without_val = xerj_mcp::tool_specs_with(false);
        let upstream_without: Vec<Tool> = serde_json::from_value(upstream_without_val)
            .expect("deserialize tool_specs_with(false)");
        assert_eq!(upstream_without.len(), 20);
        assert!(
            !upstream_without
                .iter()
                .any(|t| t.name.as_ref().starts_with("xerj_hub_"))
        );

        let upstream_with_val = xerj_mcp::tool_specs_with(true);
        let upstream_with: Vec<Tool> =
            serde_json::from_value(upstream_with_val).expect("deserialize tool_specs_with(true)");
        assert_eq!(upstream_with.len(), 23);
        assert!(
            upstream_with
                .iter()
                .any(|t| t.name.as_ref() == "xerj_hub_find")
        );

        let provider =
            XerjToolProvider::new("http://127.0.0.1:9200", None).expect("create provider");
        let expected = if provider.tools().len() == 23 {
            &upstream_with
        } else {
            &upstream_without
        };
        assert_eq!(provider.tools().len(), expected.len());
        for (exposed, spec) in provider.tools().iter().zip(expected.iter()) {
            assert_eq!(exposed.name, spec.name);
        }
    }
}
