use std::{net::SocketAddr, sync::Arc};

use log::{error, info, warn};
use memvid_core::Memvid;
use rmcp::{
    ServiceExt, transport,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
    },
};

use crate::{
    books::{BookCatalog, sync_to_memvid as sync_books_to_memvid},
    config::{AppConfig, TransportMode},
    service::MemvidService,
    skills::{SkillCatalog, sync_to_memvid as sync_skills_to_memvid},
};

const HTTP_PATH: &str = "/mcp";

/// 同步 Skill 与电子书，必要时通过配置的传输方式处理 MCP 请求。
///
/// # Errors
///
/// Skill/电子书扫描或同步失败、无法创建存储、无法启动 MCP 传输，或运行中的服务异常终止时返回错误。
pub async fn run(config: AppConfig) -> eyre::Result<()> {
    let mut memvid = if config.storage.memory.exists() {
        Memvid::open(&config.storage.memory)?
    } else {
        Memvid::create(&config.storage.memory)?
    };
    let skill_catalog = SkillCatalog::scan(&config.storage.skills)?;
    let skill_report = sync_skills_to_memvid(&mut memvid, &skill_catalog)?;
    info!(
        "skills loaded: memory={}, skills={}, loaded_skills={}, written_parts={}, unchanged_parts={}, deleted_parts={}",
        config.storage.memory.display(),
        config.storage.skills.display(),
        skill_report.skill_count,
        skill_report.written_parts,
        skill_report.unchanged_parts,
        skill_report.deleted_parts
    );

    let book_catalog = BookCatalog::scan(&config.storage.books)?;
    let book_report = sync_books_to_memvid(&mut memvid, &book_catalog)?;
    info!(
        "books loaded: memory={}, books={}, loaded_books={}, written_parts={}, unchanged_parts={}, deleted_parts={}",
        config.storage.memory.display(),
        config.storage.books.display(),
        book_report.book_count,
        book_report.written_parts,
        book_report.unchanged_parts,
        book_report.deleted_parts
    );

    match memvid.stats() {
        Ok(memory_stats) => {
            let inactive_frames = memory_stats
                .frame_count
                .saturating_sub(memory_stats.active_frame_count);
            info!(
                "memory statistics: memory={}, size_bytes={}, total_frames={}, active_frames={}, inactive_frames={}, payload_bytes={}, logical_bytes={}, saved_bytes={}, savings_percent={:.2}%, vectors={}, indexes=lex:{},vec:{},time:{},clip:{}",
                config.storage.memory.display(),
                memory_stats.size_bytes,
                memory_stats.frame_count,
                memory_stats.active_frame_count,
                inactive_frames,
                memory_stats.payload_bytes,
                memory_stats.logical_bytes,
                memory_stats.saved_bytes,
                memory_stats.savings_percent,
                memory_stats.vector_count,
                memory_stats.has_lex_index,
                memory_stats.has_vec_index,
                memory_stats.has_time_index,
                memory_stats.has_clip_index
            );
        }
        Err(error) => warn!(
            "failed to read memory statistics: memory={}, error={error}",
            config.storage.memory.display()
        ),
    }

    if config.server.sync_skills_only {
        info!("Skill and book synchronization completed; server startup disabled by configuration");
        return Ok(());
    }

    let service = MemvidService::new(memvid, skill_catalog, book_catalog);
    match config.server.transport {
        TransportMode::Stdio => {
            serve_stdio(service, skill_report.skill_count, book_report.book_count).await
        }
        TransportMode::Http => {
            serve_http(
                service,
                config.server.listen,
                skill_report.skill_count,
                book_report.book_count,
            )
            .await
        }
    }
}

async fn serve_stdio(
    service: MemvidService,
    loaded_skills: usize,
    loaded_books: usize,
) -> eyre::Result<()> {
    info!(
        "MCP server waiting: transport=stdio, loaded_skills={loaded_skills}, loaded_books={loaded_books}, protocol=stdin/stdout"
    );
    let service = service
        .serve(transport::stdio())
        .await
        .map_err(|error| eyre::eyre!("{error}"))?;

    service.waiting().await?;
    Ok(())
}

async fn serve_http(
    service: MemvidService,
    listen_address: SocketAddr,
    loaded_skills: usize,
    loaded_books: usize,
) -> eyre::Result<()> {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let cancellation_token = config.cancellation_token.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(service.clone()),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    let router = axum::Router::new().nest_service(HTTP_PATH, mcp_service);
    let listener = tokio::net::TcpListener::bind(listen_address).await?;
    let local_address = listener.local_addr()?;

    info!(
        "MCP server ready: transport=http, bind_address={local_address}, http_port={}, endpoint=http://{local_address}{HTTP_PATH}, loaded_skills={loaded_skills}, loaded_books={loaded_books}",
        local_address.port()
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            if let Err(error) = tokio::signal::ctrl_c().await {
                error!("failed to listen for shutdown signal: {error}");
            }
            cancellation_token.cancel();
        })
        .await?;
    Ok(())
}
