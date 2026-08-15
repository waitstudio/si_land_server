//! 管理员认证模块的请求 / 响应 DTO
//!
//! 字段命名遵循管理后台（vue-vben-admin）的 camelCase 约定。

use serde::{Deserialize, Serialize};

/// 管理员登录请求
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// 管理员登录响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLoginResponse {
    pub access_token: String,
}

/// 管理员用户信息（vue-vben-admin UserInfo 结构）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserInfo {
    pub user_id: String,
    pub username: String,
    pub real_name: String,
    pub roles: Vec<String>,
    pub avatar: String,
    pub home_path: String,
}
