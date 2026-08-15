//! 时间工具

use chrono::Utc;

/// 当前 Unix 时间戳（秒）
pub fn now_ts() -> i64 {
    Utc::now().timestamp()
}
