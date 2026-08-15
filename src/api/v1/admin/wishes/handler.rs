//! 管理员想看意愿查询 handler

use axum::{
    extract::{Query, State},
    Json,
};

use crate::domain::wish::StreamerWish;
use crate::error::AppError;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::WishListQuery;
use super::service::AdminWishService;

/// GET /api/v1/admin/wishes?limit=100
///
/// 想看意愿列表，按想看人数降序。
pub async fn list_wishes(
    State(state): State<AppState>,
    Query(q): Query<WishListQuery>,
) -> Result<Json<ApiResponse<Vec<StreamerWish>>>, AppError> {
    let list = AdminWishService::list_wishes(&state, q.limit.unwrap_or(100)).await?;
    Ok(Json(ApiResponse::success(list)))
}
