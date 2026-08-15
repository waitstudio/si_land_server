//! 推送凭证管理 DTO

use serde::{Deserialize, Serialize};

use crate::domain::push::PushToken;

/// 绑定 / 更新推送凭证请求
#[derive(Debug, Deserialize)]
pub struct SavePushTokenRequest {
    /// 推送通道：bark / apns
    pub channel: String,
    /// 凭证：Bark key 或 APNs device token
    pub token: String,
}

/// 推送凭证响应
#[derive(Debug, Serialize)]
pub struct PushTokenResponse {
    pub channel: String,
    pub token: String,
    pub enabled: bool,
}

impl From<PushToken> for PushTokenResponse {
    fn from(t: PushToken) -> Self {
        Self {
            channel: t.channel,
            token: t.token,
            enabled: t.enabled,
        }
    }
}
