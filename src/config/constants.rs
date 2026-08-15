//! 应用常量
//!
//! 业务语义常量集中管理，避免魔法数字与重复字面量。

/// 抖音 enter 接口 room_status == 0 表示直播中
pub const ROOM_STATUS_LIVING: i64 = 0;

/// enter 接口成功状态码
pub const ENTER_STATUS_OK: i64 = 0;

/// 热门主播列表默认 limit
pub const POPULAR_DEFAULT_LIMIT: i64 = 20;

/// 热门主播列表最大 limit
pub const POPULAR_MAX_LIMIT: i64 = 100;

/// 通知列表默认页码
pub const NOTICE_DEFAULT_PAGE: i64 = 1;

/// 通知列表默认每页条数
pub const NOTICE_DEFAULT_PAGE_SIZE: i64 = 20;

/// 通知列表每页最大条数
pub const NOTICE_MAX_PAGE_SIZE: i64 = 100;

/// 用户 ID 前缀
pub const USER_ID_PREFIX: &str = "u_";

/// 主播 ID 前缀
pub const STREAMER_ID_PREFIX: &str = "st_";

/// 开播通知 ID 前缀
pub const NOTICE_ID_PREFIX: &str = "ln_";

/// Token 类型
pub const TOKEN_TYPE: &str = "Bearer";

/// 管理员 JWT 主体（admin_guard 依据此值区分管理员与 App 用户）
pub const ADMIN_SUBJECT: &str = "admin";

/// 开播通知文案模板
pub const LIVE_NOTIFY_TEMPLATE: &str = "{} 正在直播，快来看吧";

/// 成功响应消息
pub const SUCCESS_MSG: &str = "success";
