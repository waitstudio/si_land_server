//! 请求日志中间件：记录方法、路径、状态码、耗时

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    let resp = next.run(req).await;

    tracing::info!(
        method = %method,
        path = %path,
        status = %resp.status(),
        elapsed_ms = start.elapsed().as_millis(),
        "request"
    );
    resp
}
