//! 订阅查询投影
//!
//! `SubscriptionItem` 是订阅列表查询的结果，包含主播信息与订阅时间戳。
//! 使用 `serde(flatten)` 序列化扁平化，输出 JSON 与 [crate::domain::streamer::Streamer]
//! 字段同层 + `subscribed_at`，保持前端兼容。

use serde::Serialize;
use sqlx::FromRow;

use crate::domain::streamer::Streamer;

/// 订阅列表项：主播信息 + 订阅时间
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SubscriptionItem {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub streamer: Streamer,
    /// 订阅时间戳（秒）
    pub subscribed_at: i64,
}
