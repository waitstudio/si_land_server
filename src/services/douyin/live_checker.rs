//! 开播状态检测器

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;

use super::client::DouyinEnterClient;
use super::enter_parser::EnterRoomData;

/// 开播状态
///
/// 携带抖音 enter 接口返回的可用于同步更新 streamers 表的字段，
/// 调用方按需取用，None 表示接口未返回该字段。
#[derive(Debug, Clone, Default)]
pub struct LiveStatus {
    pub is_live: bool,
    pub room_title: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

/// 开播检测器抽象
#[async_trait]
pub trait LiveChecker: Send + Sync {
    async fn check(&self, douyin_id: &str) -> Result<LiveStatus, AppError>;
}

/// 基于 [DouyinEnterClient] 的开播检测实现
pub struct HttpLiveChecker {
    client: Arc<DouyinEnterClient>,
}

impl HttpLiveChecker {
    pub fn new(client: Arc<DouyinEnterClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LiveChecker for HttpLiveChecker {
    async fn check(&self, douyin_id: &str) -> Result<LiveStatus, AppError> {
        let EnterRoomData {
            is_live,
            room_title,
            nickname,
            avatar,
            ..
        } = self.client.enter(douyin_id).await?;
        Ok(LiveStatus {
            is_live,
            room_title,
            nickname,
            avatar,
        })
    }
}
