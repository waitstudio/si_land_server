//! 开播通知业务编排

use crate::error::AppError;
use crate::state::AppState;

use super::dto::NoticeItem;

pub struct NoticeService;

impl NoticeService {
    /// 分页查询当前用户的通知列表
    pub async fn list(
        state: &AppState,
        user_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<NoticeItem>, i64, i64), AppError> {
        let (items, total) = state
            .notice_store
            .list_page(user_id, page, page_size)
            .await?;
        let unread_count = state.notice_store.unread_count(user_id).await?;
        let items = items.into_iter().map(NoticeItem::from).collect();
        Ok((items, total, unread_count))
    }

    /// 标记单条通知为已读
    ///
    /// 通知不存在或不属于该用户时返回 NotFound。
    pub async fn mark_read(
        state: &AppState,
        user_id: &str,
        notice_id: &str,
    ) -> Result<(), AppError> {
        let updated = state.notice_store.mark_read(user_id, notice_id).await?;
        if !updated {
            return Err(AppError::not_found("通知不存在或已读"));
        }
        Ok(())
    }

    /// 标记当前用户全部通知为已读，返回受影响行数
    pub async fn mark_all_read(state: &AppState, user_id: &str) -> Result<i64, AppError> {
        state.notice_store.mark_all_read(user_id).await
    }

    /// 删除单条通知
    ///
    /// 通知不存在或不属于该用户时返回 NotFound。
    pub async fn delete(state: &AppState, user_id: &str, notice_id: &str) -> Result<(), AppError> {
        let deleted = state.notice_store.delete(user_id, notice_id).await?;
        if !deleted {
            return Err(AppError::not_found("通知不存在"));
        }
        Ok(())
    }
}
