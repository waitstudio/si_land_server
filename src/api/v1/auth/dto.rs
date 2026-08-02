//! 认证模块的请求 / 响应 DTO

use serde::{Deserialize, Serialize};

use crate::domain::user::User;

/// 验证码登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub phone: String,
    pub code: String,
}

/// 验证码登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    /// 过期时间戳（秒）
    pub expires_at: i64,
    pub user: User,
}
