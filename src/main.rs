//! 服务入口：加载配置、构建状态、启动 HTTP 服务

use std::net::SocketAddr;

use si_land_server::{build_state, routes, AppConfig};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env，没有也不报错
    let _ = dotenv::dotenv();

    let config = AppConfig::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "si_land_server=debug,tower_http=debug".into()),
        )
        .init();

    let host = config.host.clone();
    let port = config.port;
    let state = build_state(config);

    let app = routes::router()
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("🚀 si_land_server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
