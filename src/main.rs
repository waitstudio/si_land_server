//! 服务入口：加载配置、构建状态、启动 HTTP 服务

use std::net::SocketAddr;

use axum::http::{HeaderValue, Method, header};
use axum::middleware::from_fn;
use si_land_server::{build_state, config::AppConfig, middleware, routes, spawn_scheduler};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    let config = AppConfig::load()?;

    /// 北京时间（UTC+8）日志时间戳，固定偏移不依赖系统 TZ 环境
    struct BeijingTime;

    impl tracing_subscriber::fmt::time::FormatTime for BeijingTime {
        fn format_time(
            &self,
            w: &mut tracing_subscriber::fmt::format::Writer<'_>,
        ) -> std::fmt::Result {
            let offset = chrono::FixedOffset::east_opt(8 * 3600).expect("合法时区偏移");
            let now = chrono::Utc::now().with_timezone(&offset);
            write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S%.3f"))
        }
    }

    let rust_log = config.server.rust_log.clone();
    tracing_subscriber::fmt()
        .with_timer(BeijingTime)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| rust_log.into()),
        )
        .init();

    let host = config.server.host.clone();
    let port = config.server.port;
    let cors = cors_layer(&config)?;
    let state = build_state(config).await?;

    // 启动轮询调度器后台任务
    spawn_scheduler(&state);

    let app = routes::router(state.clone())
        .layer(CatchPanicLayer::custom(
            middleware::catch_panic::handle_panic,
        ))
        .layer(from_fn(middleware::request_logger::request_logger))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("si_land_server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn cors_layer(config: &AppConfig) -> anyhow::Result<CorsLayer> {
    let origins = config
        .server
        .cors_allowed_origins
        .iter()
        .map(|origin| HeaderValue::from_str(origin))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]))
}
