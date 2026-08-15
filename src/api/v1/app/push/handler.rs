//! 推送凭证 handler

use axum::{extract::State, Extension, Json};
use axum::routing::{delete, post};
use axum::Router;
use serde::Serialize;

use crate::error::AppError;
use crate::middleware::auth::UserId;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{PushTokenResponse, SavePushTokenRequest};
use super::service::PushService;

/// 推送凭证管理路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/push/tokens", post(save_token).get(list_tokens))
        .route("/push/tokens/{channel}", delete(delete_token))
}

#[derive(Debug, Serialize)]
pub struct Affected {
    pub affected: usize,
}

/// POST /api/v1/app/push/tokens
pub async fn save_token(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<SavePushTokenRequest>,
) -> Result<Json<ApiResponse<Affected>>, AppError> {
    PushService::save_token(&state, &user_id.0, &req.channel, &req.token).await?;
    Ok(Json(ApiResponse::success(Affected { affected: 1 })))
}

/// GET /api/v1/app/push/tokens
pub async fn list_tokens(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<Vec<PushTokenResponse>>>, AppError> {
    let list = PushService::list_tokens(&state, &user_id.0)
        .await?
        .into_iter()
        .map(PushTokenResponse::from)
        .collect();
    Ok(Json(ApiResponse::success(list)))
}

/// DELETE /api/v1/app/push/tokens/:channel
pub async fn delete_token(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    axum::extract::Path(channel): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<Affected>>, AppError> {
    PushService::delete_token(&state, &user_id.0, &channel).await?;
    Ok(Json(ApiResponse::success(Affected { affected: 1 })))
}
