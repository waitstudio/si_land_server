//! 主播订阅 handler

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::config::constants;
use crate::domain::streamer::Streamer;
use crate::domain::subscription::SubscriptionItem;
use crate::error::AppError;
use crate::middleware::auth::UserId;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{
    CheckLiveResponse, LiveNotifyResponse, PollResponse, SubscribeRequest, WishRequest,
    WishResponse,
};
use super::service::StreamerService;

/// POST /api/v1/app/streamers
///
/// 按抖音号订阅（已停用，返回暂不支持）。保留接口兼容旧客户端。
pub async fn add_subscription(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<ApiResponse<SubscriptionItem>>, AppError> {
    let item = StreamerService::subscribe(&state, &user_id.0, &req.douyin_id).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/v1/app/streamers/:id/subscribe
///
/// 按主播 ID 订阅热门主播（主播必须已在热门列表中）。
pub async fn subscribe_by_id(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SubscriptionItem>>, AppError> {
    let item = StreamerService::subscribe_by_id(&state, &user_id.0, &id).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/v1/app/streamers/wishes
///
/// 提交想看意愿：用户输入想看的主播抖音号，按 douyin_id 去重并累加计数。
pub async fn add_wish(
    State(state): State<AppState>,
    Json(req): Json<WishRequest>,
) -> Result<Json<ApiResponse<WishResponse>>, AppError> {
    let count = StreamerService::add_wish(&state, &req.douyin_id).await?;
    Ok(Json(ApiResponse::success(WishResponse {
        want_count: count,
    })))
}

/// GET /api/v1/app/streamers
pub async fn list_subscriptions(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<Vec<SubscriptionItem>>>, AppError> {
    let list = StreamerService::list(&state, &user_id.0).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// GET /api/v1/app/streamers/popular?limit=20
#[derive(Debug, Deserialize)]
pub struct PopularQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 {
    constants::POPULAR_DEFAULT_LIMIT
}

pub async fn list_popular(
    State(state): State<AppState>,
    Query(q): Query<PopularQuery>,
) -> Result<Json<ApiResponse<Vec<Streamer>>>, AppError> {
    let limit = q.limit.clamp(1, constants::POPULAR_MAX_LIMIT);
    let list = StreamerService::list_popular(&state, limit).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// DELETE /api/v1/app/streamers/:id
pub async fn remove_subscription(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    StreamerService::unsubscribe(&state, &user_id.0, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/app/streamers/:id/check-live
pub async fn check_live(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CheckLiveResponse>>, AppError> {
    let (streamer, msg) = StreamerService::check_live(&state, &id).await?;
    Ok(Json(ApiResponse::success(CheckLiveResponse {
        live: streamer.live,
        streamer,
        message: msg,
    })))
}

/// POST /api/v1/app/streamers/poll
pub async fn poll_live(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<PollResponse>>, AppError> {
    let notifies = StreamerService::poll(&state, &user_id.0).await?;
    let list = notifies
        .into_iter()
        .map(|(streamer, message)| LiveNotifyResponse {
            streamer,
            live: true,
            message,
        })
        .collect();
    Ok(Json(ApiResponse::success(PollResponse { notifies: list })))
}
