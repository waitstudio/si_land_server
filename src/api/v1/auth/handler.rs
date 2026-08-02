//! 认证 handler

use axum::{extract::State, Json};

use crate::error::AppError;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{LoginRequest, LoginResponse};
use super::service::AuthService;

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let result = AuthService::login(&state, &req.phone, &req.code).await?;
    Ok(Json(ApiResponse::success(result)))
}
