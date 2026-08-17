//! 开播通知 handler

use axum::Router;
use axum::routing::{delete, get, post};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};

use crate::error::AppError;
use crate::middleware::auth::UserId;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{Affected, NoticeListQuery, NoticeListResponse, UnreadCountResponse};
use super::service::NoticeService;

/// 通知管理路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notices", get(list_notices))
        // 静态路径 unread-count / read-all 必须放在 {id} 之前，避免被动态路由捕获
        .route("/notices/unread-count", get(unread_count))
        .route("/notices/read-all", post(mark_all_read))
        .route("/notices/{id}/read", post(mark_read))
        .route("/notices/{id}", delete(delete_notice))
}

/// GET /api/v1/app/notices?page=1&page_size=20
pub async fn list_notices(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Query(q): Query<NoticeListQuery>,
) -> Result<Json<ApiResponse<NoticeListResponse>>, AppError> {
    let page = q.page.max(1);
    let page_size = q
        .page_size
        .clamp(1, crate::config::constants::NOTICE_MAX_PAGE_SIZE);
    let (items, total, unread_count) =
        NoticeService::list(&state, &user_id.0, page, page_size).await?;
    Ok(Json(ApiResponse::success(NoticeListResponse {
        items,
        total,
        unread_count,
    })))
}

/// GET /api/v1/app/notices/unread-count
///
/// App 冷启动 / WS 连接失败时拉取权威未读数，校准消息 Tab 红点。
pub async fn unread_count(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<UnreadCountResponse>>, AppError> {
    let count = state.notice_store.unread_count(&user_id.0).await?;
    Ok(Json(ApiResponse::success(UnreadCountResponse { count })))
}

/// POST /api/v1/app/notices/:id/read
pub async fn mark_read(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Affected>>, AppError> {
    NoticeService::mark_read(&state, &user_id.0, &id).await?;
    Ok(Json(ApiResponse::success(Affected { affected: 1 })))
}

/// POST /api/v1/app/notices/read-all
pub async fn mark_all_read(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<Affected>>, AppError> {
    let affected = NoticeService::mark_all_read(&state, &user_id.0).await?;
    Ok(Json(ApiResponse::success(Affected { affected })))
}

/// DELETE /api/v1/app/notices/:id
pub async fn delete_notice(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Affected>>, AppError> {
    NoticeService::delete(&state, &user_id.0, &id).await?;
    Ok(Json(ApiResponse::success(Affected { affected: 1 })))
}
