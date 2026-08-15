//! 主播想看意愿领域模型

use serde::Serialize;
use sqlx::FromRow;

/// 用户想看但尚未收录的主播
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StreamerWish {
    /// 抖音号（去重 key）
    pub douyin_id: String,
    /// 想看人数
    pub want_count: i64,
    /// 首次提交时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最近提交时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
