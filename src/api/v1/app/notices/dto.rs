//! 开播通知模块的请求 / 响应 DTO

use serde::{Deserialize, Serialize};

use crate::domain::notice::LiveNotice;

/// 通知列表分页查询参数
#[derive(Debug, Deserialize)]
pub struct NoticeListQuery {
    /// 页码，从 1 开始
    #[serde(default = "default_page")]
    pub page: i64,
    /// 每页条数
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    crate::config::constants::NOTICE_DEFAULT_PAGE
}

fn default_page_size() -> i64 {
    crate::config::constants::NOTICE_DEFAULT_PAGE_SIZE
}

/// 单条通知响应
#[derive(Debug, Serialize)]
pub struct NoticeItem {
    pub id: String,
    pub streamer_id: String,
    pub streamer_nickname: String,
    pub avatar: Option<String>,
    pub title: String,
    pub body: String,
    pub live_started_at: Option<i64>,
    pub created_at: i64,
    pub read: bool,
}

impl From<LiveNotice> for NoticeItem {
    fn from(n: LiveNotice) -> Self {
        Self {
            id: n.id,
            streamer_id: n.streamer_id,
            streamer_nickname: n.streamer_nickname,
            avatar: n.avatar,
            title: n.title,
            body: n.body,
            live_started_at: n.live_started_at,
            created_at: n.created_at,
            read: n.read,
        }
    }
}

/// 通知列表响应
#[derive(Debug, Serialize)]
pub struct NoticeListResponse {
    pub items: Vec<NoticeItem>,
    pub total: i64,
    pub unread_count: i64,
}

/// 未读数响应（App 冷启动 / WS 不可用时拉取）
#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

/// 受影响行数响应
#[derive(Debug, Serialize)]
pub struct Affected {
    pub affected: i64,
}
