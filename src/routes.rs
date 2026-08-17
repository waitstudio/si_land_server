//! 路由聚合

use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::api;
use crate::response::ApiResponse;
use crate::state::AppState;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

/// GET /health
async fn health() -> Json<ApiResponse<Health>> {
    Json(ApiResponse::success(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// 构建全部路由
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(api::router(state))
}
