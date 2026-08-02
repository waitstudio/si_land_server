//! 统一响应体与业务码

use serde::Serialize;

/// 统一业务码
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum BizCode {
    Success = 0,
    InvalidParam = 1001,
    SmsCodeInvalid = 1002,
    SmsCodeExpired = 1003,
    PhoneNotFound = 1004,
    Unauthorized = 1005,
    InternalError = 5000,
}

/// 统一响应体
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: BizCode::Success as i32,
            msg: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(code: BizCode, msg: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            msg: msg.into(),
            data: None,
        }
    }
}
