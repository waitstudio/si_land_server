//! 推送通道抽象
//!
//! 通过 [PushProvider] trait 隔离具体推送实现，业务层只依赖抽象。
//! 新增通道（APNs / FCM）时实现 trait 即可，无需改动业务代码。

use async_trait::async_trait;

use crate::error::AppError;

/// 推送消息载荷
#[derive(Debug, Clone)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    /// 可选跳转 URL
    pub url: Option<String>,
}

/// 推送通道：负责向指定 token 发送通知
#[async_trait]
pub trait PushProvider: Send + Sync {
    /// 通道名称（'bark' / 'apns'）
    fn channel(&self) -> &'static str;

    /// 发送一条推送
    async fn send(&self, token: &str, msg: &PushMessage) -> Result<(), AppError>;
}
