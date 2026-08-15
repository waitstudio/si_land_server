//! 抖音号 → 主播信息解析器
//!
//! 抖音官方不提供"抖音号 → sec_uid"的公开转换接口。
//! [HttpStreamerResolver] 复用 [super::client::DouyinEnterClient] 请求 enter 接口，
//! 一次性获取 sec_uid / nickname / avatar，无需额外请求与签名。

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::utils::douyin_id;

use super::client::DouyinEnterClient;
use super::enter_parser::EnterRoomData;

/// 抖音号解析结果
#[derive(Debug, Clone)]
pub struct ResolvedStreamer {
    pub sec_uid: String,
    pub nickname: String,
    pub avatar: String,
    pub is_live: bool,
}

/// 抖音号 → 主播信息解析器抽象
#[async_trait]
pub trait StreamerResolver: Send + Sync {
    async fn resolve(&self, douyin_id: &str) -> Result<ResolvedStreamer, AppError>;
}

/// 基于 [DouyinEnterClient] 的真实解析实现
pub struct HttpStreamerResolver {
    client: Arc<DouyinEnterClient>,
}

impl HttpStreamerResolver {
    pub fn new(client: Arc<DouyinEnterClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl StreamerResolver for HttpStreamerResolver {
    async fn resolve(&self, douyin_id_input: &str) -> Result<ResolvedStreamer, AppError> {
        douyin_id::validate(douyin_id_input)?;
        let EnterRoomData {
            is_live,
            sec_uid,
            nickname,
            avatar,
            ..
        } = self.client.enter(douyin_id_input).await?;

        // sec_uid 是订阅去重的关键，缺失视为解析失败
        let sec_uid = sec_uid.ok_or_else(|| {
            AppError::invalid_param("无法解析该抖音号的 sec_uid，请确认抖音号正确")
        })?;

        Ok(ResolvedStreamer {
            sec_uid,
            nickname: nickname.unwrap_or_else(|| douyin_id_input.to_string()),
            avatar: avatar.unwrap_or_default(),
            is_live,
        })
    }
}
