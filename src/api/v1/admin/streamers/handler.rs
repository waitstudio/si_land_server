//! 管理员主播收录 handler

use axum::{extract::State, Json};

use crate::domain::streamer::Streamer;
use crate::error::AppError;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::AddStreamerRequest;
use super::service::AdminStreamerService;

/// POST /api/v1/admin/streamers
///
/// 管理员按抖音号收录主播（原 App 端手动订阅逻辑）。
pub async fn add_streamer(
    State(state): State<AppState>,
    Json(req): Json<AddStreamerRequest>,
) -> Result<Json<ApiResponse<Streamer>>, AppError> {
    let streamer = AdminStreamerService::add_streamer(&state, &req.douyin_id).await?;
    Ok(Json(ApiResponse::success(streamer)))
}

/// GET /api/v1/admin/streamers
///
/// 已收录主播列表（按人气降序）。
pub async fn list_streamers(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Streamer>>>, AppError> {
    let list = AdminStreamerService::list_streamers(&state).await?;
    Ok(Json(ApiResponse::success(list)))
}
