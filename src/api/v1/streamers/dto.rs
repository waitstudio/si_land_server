//! 主播订阅模块的请求 / 响应 DTO

use serde::{Deserialize, Serialize};

use crate::domain::streamer::Streamer;

/// 添加订阅请求
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    /// 抖音号（用户自定义短号，非链接 / 分享口令）
    pub douyin_id: String,
}

/// mock 触发开播通知响应
#[derive(Debug, Serialize)]
pub struct LiveNotifyResponse {
    pub streamer: Streamer,
    pub live: bool,
    pub message: String,
}

/// 检测开播响应
#[derive(Debug, Serialize)]
pub struct CheckLiveResponse {
    pub streamer: Streamer,
    pub live: bool,
    /// 若"未播→在播"，返回通知文案；否则为 None
    pub message: Option<String>,
}

/// 批量轮询响应
#[derive(Debug, Serialize)]
pub struct PollResponse {
    /// 新开播的主播通知列表
    pub notifies: Vec<LiveNotifyResponse>,
}
