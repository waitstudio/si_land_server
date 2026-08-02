//! 短信模块的请求 / 响应 DTO

use serde::{Deserialize, Serialize};

/// 发送验证码请求
#[derive(Debug, Deserialize)]
pub struct SendSmsRequest {
    pub phone: String,
}

/// 发送验证码响应
#[derive(Debug, Serialize)]
pub struct SendSmsResponse {
    pub phone: String,
    /// 验证码有效期（秒）
    pub expire_in: u64,
}
