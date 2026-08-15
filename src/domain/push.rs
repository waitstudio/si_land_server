//! 推送领域模型

use sqlx::FromRow;

/// 用户绑定的推送凭证（一个用户可多通道）
#[derive(Debug, Clone, FromRow)]
pub struct PushToken {
    pub user_id: String,
    /// 'bark' / 'apns'
    pub channel: String,
    /// Bark key 或 APNs device token
    pub token: String,
    pub enabled: bool,
}
