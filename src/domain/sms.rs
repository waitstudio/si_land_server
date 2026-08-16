//! 短信验证码领域模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 一条验证码记录
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::SmsCode;

    #[test]
    fn expires_at_boundary() {
        let now = Utc::now();
        let code = SmsCode {
            phone: "13800138000".to_string(),
            code: "123456".to_string(),
            created_at: now - Duration::seconds(60),
            expire_at: now,
        };
        assert!(code.is_expired(now));
    }
}
