//! 管理员想看意愿查询业务编排

use crate::config::constants;
use crate::domain::wish::StreamerWish;
use crate::error::AppError;
use crate::state::AppState;

pub struct AdminWishService;

impl AdminWishService {
    /// 想看意愿列表（按想看人数降序）
    pub async fn list_wishes(
        state: &AppState,
        limit: i64,
    ) -> Result<Vec<StreamerWish>, AppError> {
        let limit = limit.clamp(1, constants::POPULAR_MAX_LIMIT);
        state.wish_store.list_wishes(limit).await
    }
}
