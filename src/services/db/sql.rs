//! SQL 片段常量
//!
//! 集中表名与列名，消除 5 处重复的 SELECT 列列表。

/// streamers 表所有列（不含 subscribed_at，那是订阅投影字段）
pub const STREAMER_COLUMNS: &str = "id, sec_uid, douyin_id, nickname, avatar, live, \
    live_started_at, popularity";
