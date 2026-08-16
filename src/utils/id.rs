//! ID 生成：ULID（128 位，时间戳有序）

use crate::config::constants;

/// 生成用户 ID：`u_` + ULID
pub fn gen_user_id() -> String {
    format!("{}{}", constants::USER_ID_PREFIX, ulid::Ulid::new().to_string().to_lowercase())
}

/// 生成主播 ID：`st_` + ULID
pub fn gen_streamer_id() -> String {
    format!("{}{}", constants::STREAMER_ID_PREFIX, ulid::Ulid::new().to_string().to_lowercase())
}

/// 生成开播通知 ID：`ln_` + ULID
pub fn gen_notice_id() -> String {
    format!("{}{}", constants::NOTICE_ID_PREFIX, ulid::Ulid::new().to_string().to_lowercase())
}

/// 生成问题反馈 ID：`fb_` + ULID
pub fn gen_feedback_id() -> String {
    format!("{}{}", constants::FEEDBACK_ID_PREFIX, ulid::Ulid::new().to_string().to_lowercase())
}
