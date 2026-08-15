//! 认证 handler

use axum::{extract::State, Json};
use axum::Extension;

use crate::error::AppError;
use crate::middleware::auth::UserId;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{LoginRequest, LoginResponse, UpdateNicknameRequest};
use super::service::AuthService;

/// POST /api/v1/app/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let result = AuthService::login(&state, &req.phone, &req.code).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/app/auth/me
///
/// 需携带 Authorization: Bearer <token>，中间件已校验并注入 UserId。
pub async fn me(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiResponse<crate::domain::user::User>>, AppError> {
    let user = AuthService::me(&state, &user_id.0).await?;
    Ok(Json(ApiResponse::success(user)))
}

/// PUT /api/v1/app/auth/nickname
///
/// 修改当前登录用户的昵称。
pub async fn update_nickname(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<UpdateNicknameRequest>,
) -> Result<Json<ApiResponse<crate::domain::user::User>>, AppError> {
    let user = AuthService::update_nickname(&state, &user_id.0, &req.nickname).await?;
    Ok(Json(ApiResponse::success(user)))
}
