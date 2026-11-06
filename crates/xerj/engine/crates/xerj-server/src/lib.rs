//! Library bootstrap entry points for embedding a full XERJ server runtime.
//!
//! Exposes the same Native REST, ES-compat (with Corpus Hub), Xerj Console,
//! and gRPC (`XerjSearch` + `XerjHub`) listener orchestration used by the `xerj`
//! binary so host applications can share an in-process [`Engine`] instance.

use std::net::SocketAddr;
#[cfg(feature = "console")]
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
#[cfg(feature = "console")]
use tracing::warn;
use tracing::{error, info};
use xerj_api::{build_es_compat_router_with_hub, build_native_router, AppState};
use xerj_common::{config::Config, metrics::Metrics};
#[cfg(feature = "console")]
use xerj_console_api::{state::ClusterMode, ConsoleState, RpConfig};
use xerj_engine::Engine;

pub mod grpc;
pub mod grpc_hub;

/// Bound listener addresses for a running embedded XERJ server.
#[derive(Debug, Clone, Copy)]
pub struct BoundServerAddrs {
    /// Bound address for the Native REST API listener.
    pub rest: SocketAddr,
    /// Bound address for the Elasticsearch-compatible API listener.
    pub es_compat: SocketAddr,
    /// Bound address for the gRPC (`XerjSearch` + `XerjHub`) listener.
    pub grpc: SocketAddr,
}

/// Handle to a running XERJ server started via [`start_with_engine`].
pub struct EmbeddedServer {
    addrs: BoundServerAddrs,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::task::JoinHandle<Result<()>>,
}

impl EmbeddedServer {
    /// Return the bound TCP addresses of all three XERJ listeners.
    pub fn addrs(&self) -> BoundServerAddrs {
        self.addrs
    }

    /// Return the base `http://host:port` URL of the ES-compatible listener.
    pub fn es_base_url(&self) -> String {
        format!("http://{}", self.addrs.es_compat)
    }

    /// Signal the server to begin graceful shutdown and wait for completion.
    ///
    /// # Errors
    ///
    /// Returns an error if the background task panicked or reported a fatal shutdown error.
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.shutdown_tx.send(true);
        self.join_handle
            .await
            .context("join embedded xerj-server")?
    }

    /// Wait until the server exits (for example, on Ctrl-C / SIGTERM or explicit shutdown).
    ///
    /// # Errors
    ///
    /// Returns an error if the server task fails.
    pub async fn wait(self) -> Result<()> {
        self.join_handle
            .await
            .context("join embedded xerj-server")?
    }
}

/// Start the full XERJ server runtime (Native REST, ES-compat + Hub, Console, gRPC,
/// background flusher, resource sampler, lifecycle manager, and WAL tap) over an
/// existing [`Engine`] instance.
///
/// # Errors
///
/// Returns an error if listener binding, metrics creation, or console bootstrap fails.
pub async fn start_with_engine(mut cfg: Config, engine: Arc<Engine>) -> Result<EmbeddedServer> {
    let ip = cfg
        .bind_ip()
        .with_context(|| format!("invalid server.bind_address: {}", cfg.server.bind_address))?;

    let es_std = std::net::TcpListener::bind(SocketAddr::new(ip, cfg.server.es_compat_port))
        .with_context(|| {
            format!(
                "bind ES-compat listener on {}:{}",
                ip, cfg.server.es_compat_port
            )
        })?;
    es_std.set_nonblocking(true)?;
    let es_addr = es_std.local_addr()?;

    let rest_std = std::net::TcpListener::bind(SocketAddr::new(ip, cfg.server.rest_port))
        .with_context(|| {
            format!(
                "bind native REST listener on {}:{}",
                ip, cfg.server.rest_port
            )
        })?;
    rest_std.set_nonblocking(true)?;
    let rest_addr = rest_std.local_addr()?;

    let grpc_std = std::net::TcpListener::bind(SocketAddr::new(ip, cfg.server.grpc_port))
        .with_context(|| format!("bind gRPC listener on {}:{}", ip, cfg.server.grpc_port))?;
    grpc_std.set_nonblocking(true)?;
    let grpc_addr = grpc_std.local_addr()?;

    cfg.server.es_compat_port = es_addr.port();
    cfg.server.rest_port = rest_addr.port();
    cfg.server.grpc_port = grpc_addr.port();

    let metrics = Metrics::new().context("initialise XERJ metrics")?;
    let storage_available = engine.cluster_state_boot_status().is_writable();

    #[cfg(feature = "console")]
    let xerj_console_state = {
        let trusted_proxies = xerj_common::net::TrustedProxies::parse(&cfg.server.trusted_proxies)
            .map_err(|why| anyhow::anyhow!("server.trusted_proxies: {why}"))?;

        let xerj_console_bind_url = format!(
            "http://{}:{}",
            xerj_console_api::public_bind_host(&cfg.server.bind_address),
            es_addr.port(),
        );

        if storage_available {
            let outcome = xerj_console_api::bootstrap::run(
                &engine,
                Path::new(&cfg.server.data_dir),
                &xerj_console_bind_url,
            )
            .await
            .context("xerj-console bootstrap")?;
            let console_rp = match RpConfig::from_bind_url(&xerj_console_bind_url) {
                Ok(rp) => rp,
                Err(why) => {
                    warn!("Xerj Console passkey sign-in is unavailable: {why}");
                    RpConfig::default()
                }
            };
            Some(
                ConsoleState::new_with_rp(
                    engine.clone(),
                    "local".to_string(),
                    outcome.master_key,
                    ClusterMode::Standalone,
                    console_rp,
                )
                .with_trusted_proxies(trusted_proxies),
            )
        } else {
            None
        }
    };

    let state = AppState::new(cfg.clone(), (*engine).clone(), metrics);
    xerj_engine::set_engine_metrics(state.metrics.clone());

    {
        let tap = state.engine.wal_tap.clone();
        let tap_engine = state.engine.clone();
        tokio::spawn(xerj_engine::wal_tap::run(tap, tap_engine));
    }

    #[allow(unused_mut)]
    let mut native_router = build_native_router(state.clone());
    let hub_shared = xerj_api::hub_api::HubShared::new(state.clone());
    #[allow(unused_mut)]
    let mut es_router = build_es_compat_router_with_hub(state.clone(), hub_shared.clone());
    #[cfg(feature = "console")]
    if let Some(console_state) = xerj_console_state {
        let console_router = xerj_console_api::xerj_console_router(console_state);
        native_router = native_router.merge(console_router.clone());
        es_router = es_router.merge(console_router);
    }

    let flusher = storage_available.then(|| {
        let flush_engine = state.engine.clone();
        let interval = Duration::from_secs(cfg.storage.flush_interval_secs.max(1));
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                flush_engine.flush_all_if_needed().await;
            }
        })
    });

    if storage_available {
        state.engine.spawn_resource_sampler();
        state.engine.spawn_lifecycle_manager();
    }

    let es_listener = TcpListener::from_std(es_std)?;
    let rest_listener = TcpListener::from_std(rest_std)?;

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let mut rest_shutdown = shutdown_rx.clone();
    let mut es_shutdown = shutdown_rx.clone();
    let mut grpc_shutdown = shutdown_rx.clone();

    let mut rest_task = tokio::spawn(async move {
        let svc = native_router.into_make_service_with_connect_info::<SocketAddr>();
        if let Err(err) = axum::serve(rest_listener, svc)
            .with_graceful_shutdown(async move {
                let _ = rest_shutdown.wait_for(|&v| v).await;
            })
            .await
        {
            error!("native REST listener error: {err:#}");
        }
    });

    let mut es_task = tokio::spawn(async move {
        let svc = es_router.into_make_service_with_connect_info::<SocketAddr>();
        if let Err(err) = axum::serve(es_listener, svc)
            .with_graceful_shutdown(async move {
                let _ = es_shutdown.wait_for(|&v| v).await;
            })
            .await
        {
            error!("ES-compat listener error: {err:#}");
        }
    });

    let grpc_state = state.clone();
    let mut grpc_task = tokio::spawn(async move {
        if let Err(err) = grpc::serve_grpc_on(grpc_std, grpc_state, hub_shared, async move {
            let _ = grpc_shutdown.wait_for(|&v| v).await;
        })
        .await
        {
            error!("gRPC listener error: {err:#}");
        }
    });

    let internal_shutdown_tx = shutdown_tx.clone();
    let join_handle = tokio::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let _ = internal_shutdown_tx.send(true);
            }
            _ = shutdown_rx.wait_for(|&v| v) => {}
            _ = &mut rest_task => {}
            _ = &mut es_task => {}
            _ = &mut grpc_task => {}
        }

        if let Some(flusher) = flusher {
            flusher.abort();
        }

        if storage_available {
            info!("flushing in-memory XERJ state before exit…");
            let _ =
                tokio::time::timeout(Duration::from_secs(10), state.engine.flush_all_force()).await;
        }

        let _ = internal_shutdown_tx.send(true);
        let _ = rest_task.await;
        let _ = es_task.await;
        let _ = grpc_task.await;
        Ok(())
    });

    Ok(EmbeddedServer {
        addrs: BoundServerAddrs {
            rest: rest_addr,
            es_compat: es_addr,
            grpc: grpc_addr,
        },
        shutdown_tx,
        join_handle,
    })
}
