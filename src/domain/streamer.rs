//! 抖音主播领域模型

use serde::Serialize;
use sqlx::FromRow;

/// 抖音主播
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Streamer {
    /// 主播唯一标识（st_ 前缀 + ULID）
    pub id: String,
    /// 抖音 sec_uid（订阅去重 key）
    pub sec_uid: String,
    /// 抖音号短号
    pub douyin_id: String,
    /// 主播昵称
    pub nickname: String,
    /// 头像 URL
    pub avatar: String,
    /// 是否正在直播
    pub live: bool,
    /// 开播时间戳（秒），未在播为 None
    pub live_started_at: Option<i64>,
    /// 人气值（被订阅次数）
    pub popularity: i64,
}
