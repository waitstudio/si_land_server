//! 统一响应体与业务码

use serde::Serialize;

use crate::config::constants;

/// 统一业务码
#[derive(Debug, Clone, Copy)]
pub enum BizCode {
    Success = 0,
    InvalidParam = 1001,
    SmsCodeInvalid = 1002,
    SmsCodeExpired = 1003,
    PhoneNotFound = 1004,
    Unauthorized = 1005,
    NotFound = 1006,
    Conflict = 1007,
    RateLimit = 1008,
    InternalError = 5000,
}

impl BizCode {
    /// 映射到 HTTP 状态码
    pub fn http_status(self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::Success => StatusCode::OK,
            Self::InvalidParam
            | Self::SmsCodeInvalid
            | Self::SmsCodeExpired
            | Self::PhoneNotFound => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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
            msg: constants::SUCCESS_MSG.to_string(),
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
