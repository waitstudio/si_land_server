//! 短信 handler

use axum::{extract::State, Json};

use crate::error::AppError;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{SendSmsRequest, SendSmsResponse};
use super::service::SmsService;

/// POST /api/v1/sms/send
pub async fn send_sms(
    State(state): State<AppState>,
    Json(req): Json<SendSmsRequest>,
) -> Result<Json<ApiResponse<SendSmsResponse>>, AppError> {
    let expire_in = SmsService::send_code(&state, &req.phone).await?;
    Ok(Json(ApiResponse::success(SendSmsResponse {
        phone: req.phone,
        expire_in,
    })))
}
