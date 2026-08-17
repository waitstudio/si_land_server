//! panic 兜底处理器
//!
//! 配合 `tower_http::catch_panic::CatchPanicLayer::custom` 使用，
//! 把 panic 转成统一 ApiResponse 格式，防止单个请求 panic 断连其他请求。

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::response::{ApiResponse, BizCode};

/// panic 响应处理器
pub fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "未知 panic".to_string()
    };
    tracing::error!("请求处理 panic: {msg}");
    let body = ApiResponse::<serde_json::Value>::error(
        BizCode::InternalError,
        "服务暂时不可用，请稍后再试",
    );
    (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
}
