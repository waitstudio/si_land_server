//! 开播通知领域模型
//!
//! 纯业务数据结构，不依赖 HTTP / IO。

use sqlx::FromRow;

/// 开播通知实体（对应 live_notices 表一行）
///
/// `avatar` 由查询时 LEFT JOIN streamers 表获取，不入库存储。
#[derive(Debug, Clone, FromRow)]
pub struct LiveNotice {
    /// `ln_` + ULID
    pub id: String,
    pub user_id: String,
    pub streamer_id: String,
    /// 主播昵称快照（入库时刻的昵称，避免后续改名错位）
    pub streamer_nickname: String,
    pub title: String,
    pub body: String,
    /// 主播开播时间（秒），可能为空
    pub live_started_at: Option<i64>,
    /// 消息产生时间（秒）
    pub created_at: i64,
    pub read: bool,
    /// 主播头像 URL（JOIN streamers 表获取，可能为空）
    #[sqlx(default)]
    pub avatar: Option<String>,
}
