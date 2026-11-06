//! Application startup, catalog synchronization, and runtime mode orchestration for Amemory.

pub mod config;

use std::sync::Arc;

use amemory_adapter::{SearchStore, XerjAdapter};
use amemory_domain::book::BookCatalog;
use amemory_domain::memory::IndexTarget;
use amemory_domain::skill::SkillCatalog;
use amemory_mcp::{
    AmemoryToolProvider, HttpMcpConfig, UnifiedMcpServer, XerjToolProvider, serve_http, serve_stdio,
};
use eyre::{Result, WrapErr};
use tracing::info;

pub use config::{
    AppConfig, DocumentImportConfig, ServerConfig, ServerMode, StorageConfig, XerjSectionConfig,
};

/// Execute the Amemory application lifecycle according to `config`.
///
/// # Errors
///
/// Returns an error if configuration validation, XERJ engine initialization,
/// catalog synchronization, or MCP/Server runtime execution fails.
pub async fn run(config: AppConfig) -> Result<()> {
    config.validate()?;

    let skills_catalog = SkillCatalog::new(&config.storage.skills);
    let books_catalog = BookCatalog::new(&config.storage.books);

    let adapter = Arc::new(
        XerjAdapter::open(config.to_xerj_config()).wrap_err("初始化 XERJ 引擎适配器失败")?,
    );

    let (scanned_skills, skill_docs) = skills_catalog
        .build_all_index_documents()
        .wrap_err("扫描并构建 Skill 索引文档失败")?;
    let skill_sync = adapter
        .sync_catalog(IndexTarget::Skills, scanned_skills, &skill_docs)
        .await
        .wrap_err("同步 Skill 目录到 XERJ 失败")?;
    info!("{}", skill_sync.message);

    let (scanned_books, book_docs) = books_catalog
        .build_all_index_documents()
        .wrap_err("扫描并构建 Book 索引文档失败")?;
    let book_sync = adapter
        .sync_catalog(IndexTarget::Books, scanned_books, &book_docs)
        .await
        .wrap_err("同步 Book 目录到 XERJ 失败")?;
    info!("{}", book_sync.message);

    match config.server.mode {
        ServerMode::Sync => run_sync_mode(&adapter).await,
        ServerMode::McpStdio => {
            run_mcp_stdio(&config, adapter, skills_catalog, books_catalog).await
        }
        ServerMode::McpHttp => run_mcp_http(&config, adapter, skills_catalog, books_catalog).await,
        ServerMode::Server => {
            run_full_server(&config, adapter, skills_catalog, books_catalog).await
        }
    }
}

async fn run_sync_mode(adapter: &XerjAdapter) -> Result<()> {
    let stats = adapter.stats().await.wrap_err("读取索引统计信息失败")?;
    info!(
        skills_docs = stats.skills_docs,
        books_docs = stats.books_docs,
        memory_docs = stats.memory_docs,
        data_dir = %stats.data_dir,
        "同步模式完成，正在刷盘退出"
    );
    adapter.flush_all().await;
    Ok(())
}

async fn run_mcp_stdio(
    config: &AppConfig,
    adapter: Arc<XerjAdapter>,
    skills_catalog: SkillCatalog,
    books_catalog: BookCatalog,
) -> Result<()> {
    let loopback = if config.xerj.external_es_url.is_none() {
        Some(
            adapter
                .start_loopback_es_listener()
                .await
                .wrap_err("启动本地回环 XERJ ES 监听器失败")?,
        )
    } else {
        None
    };
    let es_url = config
        .xerj
        .external_es_url
        .as_deref()
        .or_else(|| {
            loopback
                .as_ref()
                .map(amemory_adapter::LoopbackEsServer::base_url)
        })
        .unwrap_or("http://127.0.0.1:9200");

    let unified = build_unified_mcp(
        config,
        adapter.clone(),
        skills_catalog,
        books_catalog,
        es_url,
    )?;
    let res = serve_stdio(unified)
        .await
        .wrap_err("MCP stdio 服务运行失败");
    if let Some(loopback) = loopback {
        loopback.shutdown().await;
    }
    adapter.flush_all().await;
    res
}

async fn run_mcp_http(
    config: &AppConfig,
    adapter: Arc<XerjAdapter>,
    skills_catalog: SkillCatalog,
    books_catalog: BookCatalog,
) -> Result<()> {
    let loopback = if config.xerj.external_es_url.is_none() {
        Some(
            adapter
                .start_loopback_es_listener()
                .await
                .wrap_err("启动本地回环 XERJ ES 监听器失败")?,
        )
    } else {
        None
    };
    let es_url = config
        .xerj
        .external_es_url
        .as_deref()
        .or_else(|| {
            loopback
                .as_ref()
                .map(amemory_adapter::LoopbackEsServer::base_url)
        })
        .unwrap_or("http://127.0.0.1:9200");

    let unified = build_unified_mcp(
        config,
        adapter.clone(),
        skills_catalog,
        books_catalog,
        es_url,
    )?;
    let http_cfg = build_http_mcp_config(config)?;

    info!(
        bind_addr = %http_cfg.bind_addr,
        endpoint = %http_cfg.endpoint_path,
        "启动统一 MCP HTTP 服务"
    );
    let res = serve_http(unified, http_cfg)
        .await
        .wrap_err("MCP HTTP 服务运行失败");

    if let Some(loopback) = loopback {
        loopback.shutdown().await;
    }
    adapter.flush_all().await;
    res
}

async fn run_full_server(
    config: &AppConfig,
    adapter: Arc<XerjAdapter>,
    skills_catalog: SkillCatalog,
    books_catalog: BookCatalog,
) -> Result<()> {
    let running_xerj = adapter
        .start_full_server()
        .await
        .wrap_err("启动内嵌完整 XERJ Server 失败")?;

    let es_url = config
        .xerj
        .external_es_url
        .clone()
        .unwrap_or_else(|| running_xerj.es_base_url());

    let unified = build_unified_mcp(
        config,
        adapter.clone(),
        skills_catalog,
        books_catalog,
        &es_url,
    )?;
    let http_cfg = build_http_mcp_config(config)?;

    info!(
        mcp_addr = %http_cfg.bind_addr,
        mcp_endpoint = %http_cfg.endpoint_path,
        xerj_es = %running_xerj.es_compat_addr(),
        xerj_rest = %running_xerj.rest_addr(),
        xerj_grpc = %running_xerj.grpc_addr(),
        "Amemory + XERJ 完整服务已启动"
    );

    let mcp_res = serve_http(unified, http_cfg)
        .await
        .wrap_err("MCP HTTP 服务运行失败");
    running_xerj
        .shutdown()
        .await
        .wrap_err("关闭内嵌 XERJ Server 失败")?;
    adapter.flush_all().await;
    mcp_res
}

fn build_unified_mcp(
    config: &AppConfig,
    adapter: Arc<XerjAdapter>,
    skills_catalog: SkillCatalog,
    books_catalog: BookCatalog,
    es_url: &str,
) -> Result<UnifiedMcpServer<XerjAdapter>> {
    let amemory_provider = AmemoryToolProvider::new(
        adapter,
        skills_catalog,
        books_catalog,
        config.to_import_policy(),
    );
    let xerj_provider = XerjToolProvider::new(es_url, config.resolve_hub_api_key())
        .wrap_err("初始化 XERJ MCP Provider 失败")?;
    UnifiedMcpServer::new(amemory_provider, xerj_provider).wrap_err("构建统一 MCP 服务失败")
}

fn build_http_mcp_config(config: &AppConfig) -> Result<HttpMcpConfig> {
    Ok(HttpMcpConfig {
        bind_addr: config.server.socket_addr()?,
        endpoint_path: config.server.endpoint_path.clone(),
        max_request_body_bytes: config.server.max_request_body_bytes,
        allow_remote: config.server.allow_remote,
    })
}

#[cfg(test)]
mod tests {
    use amemory_domain::book::{BookFileInput, CreateBookInput};
    use amemory_domain::memory::{SearchMode, SearchRequest};
    use amemory_domain::skill::CreateSkillInput;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sync_mode_runs_and_persists_indices() {
        let dir = TempDir::new().expect("tempdir");
        let mut cfg = AppConfig::default();
        cfg.server.mode = ServerMode::Sync;
        cfg.storage.skills = dir.path().join("skills");
        cfg.storage.books = dir.path().join("books");
        cfg.xerj.data_dir = dir.path().join("xerj");

        run(cfg).await.expect("run sync mode");
    }

    fn seed_sample_catalogs(skills_dir: &std::path::Path, books_dir: &std::path::Path) {
        let skills = SkillCatalog::new(skills_dir);
        skills
            .create_skill(&CreateSkillInput {
                name: Some("rust-error-handling".to_string()),
                description: Some("Rust 错误处理与 thiserror/eyre 最佳实践指南".to_string()),
                body: Some(
                    "# Rust 错误处理\n\n库代码使用 `thiserror` 枚举，应用入口使用 `eyre::Result`。"
                        .to_string(),
                ),
                skill_markdown: None,
                files: Vec::new(),
            })
            .expect("create skill");

        let books = BookCatalog::new(books_dir);
        books
            .create_book(&CreateBookInput {
                name: Some("distributed-consensus".to_string()),
                description: Some("Raft 与 Paxos 分布式一致性协议架构解析".to_string()),
                body: Some(
                    "# 分布式一致性\n\n介绍 Raft Leader Election 与 Log Replication。".to_string(),
                ),
                book_markdown: None,
                files: vec![BookFileInput {
                    relative_path: "chapters/raft.md".to_string(),
                    content: "# Raft 选主与日志复制\n\nTerm 单调递增，Heartbeat 维持 Leader 权威。"
                        .to_string(),
                }],
            })
            .expect("create book");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rebuild_recovery_and_multilingual_search_recall() {
        let dir = TempDir::new().expect("tempdir");
        let skills_dir = dir.path().join("skills");
        let books_dir = dir.path().join("books");
        let xerj_dir = dir.path().join("xerj");

        seed_sample_catalogs(&skills_dir, &books_dir);

        let mut cfg = AppConfig::default();
        cfg.server.mode = ServerMode::Sync;
        cfg.storage.skills = skills_dir.clone();
        cfg.storage.books = books_dir.clone();
        cfg.xerj.data_dir = xerj_dir.clone();

        // 1. Initial sync builds XERJ indices from filesystem truth sources.
        run(cfg.clone()).await.expect("initial sync");

        // 2. Simulate catastrophic index loss by deleting `data/xerj` completely.
        std::fs::remove_dir_all(&xerj_dir).expect("delete xerj data dir");
        assert!(!xerj_dir.exists());

        // 3. Re-running sync rebuilds indices cleanly from `./resource/skills` and `./resource/books`.
        run(cfg.clone())
            .await
            .expect("rebuild sync after deleting data_dir");
        assert!(xerj_dir.exists());

        // 4. Open adapter and verify Chinese, English, and mixed-language search recall.
        let adapter = XerjAdapter::open(cfg.to_xerj_config()).expect("open rebuilt adapter");
        let stats = adapter.stats().await.expect("stats");
        assert!(stats.skills_docs >= 1);
        assert!(stats.books_docs >= 2);

        let zh_skill_hits = adapter
            .search(&SearchRequest {
                query: "错误处理 最佳实践".to_string(),
                limit: 5,
                snippet_chars: 200,
                mode: SearchMode::Lexical,
                uri: None,
                scope: Some("skill://".to_string()),
                exclude_uris: Vec::new(),
                exclude_scopes: Vec::new(),
                tag_filters: Vec::new(),
            })
            .await
            .expect("zh skill search");
        assert!(
            !zh_skill_hits.is_empty(),
            "Chinese skill query should recall hits"
        );
        assert!(
            zh_skill_hits[0]
                .uri
                .as_deref()
                .unwrap_or_default()
                .starts_with("skill://rust-error-handling/")
        );

        let mixed_skill_hits = adapter
            .search(&SearchRequest {
                query: "thiserror 枚举".to_string(),
                limit: 5,
                snippet_chars: 200,
                mode: SearchMode::Lexical,
                uri: None,
                scope: Some("skill://".to_string()),
                exclude_uris: Vec::new(),
                exclude_scopes: Vec::new(),
                tag_filters: Vec::new(),
            })
            .await
            .expect("mixed skill search");
        assert!(
            !mixed_skill_hits.is_empty(),
            "Mixed language skill query should recall hits"
        );

        let book_hits = adapter
            .search(&SearchRequest {
                query: "Raft 选主 Leader".to_string(),
                limit: 5,
                snippet_chars: 200,
                mode: SearchMode::Lexical,
                uri: None,
                scope: Some("book://distributed-consensus/".to_string()),
                exclude_uris: Vec::new(),
                exclude_scopes: Vec::new(),
                tag_filters: Vec::new(),
            })
            .await
            .expect("book search");
        assert!(
            !book_hits.is_empty(),
            "Book query should recall rebuilt chapters"
        );
        assert!(
            book_hits[0]
                .uri
                .as_deref()
                .unwrap_or_default()
                .starts_with("book://distributed-consensus/")
        );
    }
}
