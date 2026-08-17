//! 问题反馈 handler

use axum::{Extension, Json, extract::State};

use crate::config::constants::{FEEDBACK_MAX_CONTENT_LEN, FEEDBACK_MIN_CONTENT_LEN};
use crate::error::AppError;
use crate::middleware::auth::UserId;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{FeedbackRequest, FeedbackResponse};

/// POST /api/v1/app/feedback
///
/// 提交问题反馈：内容去除首尾空白后非空且不超过 500 字符。
pub async fn submit(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<ApiResponse<FeedbackResponse>>, AppError> {
    let content = req.content.trim();
    let len = content.chars().count();
    if len < FEEDBACK_MIN_CONTENT_LEN {
        return Err(AppError::invalid_param("反馈内容不能为空"));
    }
    if len > FEEDBACK_MAX_CONTENT_LEN {
        return Err(AppError::invalid_param(format!(
            "反馈内容不能超过 {FEEDBACK_MAX_CONTENT_LEN} 字"
        )));
    }

    let id = state.feedback_store.create(&user_id.0, content).await?;
    tracing::info!(user_id = %user_id.0, feedback_id = %id, "收到问题反馈");

    Ok(Json(ApiResponse::success(FeedbackResponse { id })))
}
