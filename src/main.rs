//! 服务入口：加载配置、构建状态、启动 HTTP 服务

use std::net::SocketAddr;

use axum::middleware::from_fn;
use si_land_server::{build_state, config::AppConfig, middleware, routes, spawn_scheduler};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    let config = AppConfig::load()?;

    let rust_log = config.server.rust_log.clone();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| rust_log.into()),
        )
        .init();

    let host = config.server.host.clone();
    let port = config.server.port;
    let state = build_state(config).await?;

    // 启动轮询调度器后台任务
    spawn_scheduler(&state);

    let app = routes::router(state.clone())
        .layer(CatchPanicLayer::custom(middleware::catch_panic::handle_panic))
        .layer(from_fn(middleware::request_logger::request_logger))
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("si_land_server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
