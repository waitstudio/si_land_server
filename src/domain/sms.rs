//! 短信验证码领域模型

use chrono::{DateTime, Utc};

/// 一条验证码记录
#[derive(Debug, Clone)]
pub struct SmsCode {
    pub phone: String,
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub expire_at: DateTime<Utc>,
}

impl SmsCode {
    /// 在指定时间点是否已过期
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expire_at
    }
}
