//! 统一错误类型，实现 IntoResponse 自动转成统一响应体

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::response::{ApiResponse, BizCode};

/// 业务错误
#[derive(Debug)]
pub struct AppError {
    pub code: BizCode,
    pub msg: String,
}

impl AppError {
    pub fn new(code: BizCode, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
        }
    }

    pub fn invalid_param(msg: impl Into<String>) -> Self {
        Self::new(BizCode::InvalidParam, msg)
    }

    pub fn sms_code_invalid(msg: impl Into<String>) -> Self {
        Self::new(BizCode::SmsCodeInvalid, msg)
    }

    pub fn sms_code_expired(msg: impl Into<String>) -> Self {
        Self::new(BizCode::SmsCodeExpired, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(BizCode::InternalError, msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self.code {
            BizCode::Success => StatusCode::OK,
            BizCode::InvalidParam
            | BizCode::SmsCodeInvalid
            | BizCode::SmsCodeExpired
            | BizCode::PhoneNotFound => StatusCode::BAD_REQUEST,
            BizCode::Unauthorized => StatusCode::UNAUTHORIZED,
            BizCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = ApiResponse::<serde_json::Value>::error(self.code, self.msg);
        (status, Json(body)).into_response()
    }
}

/// 把 Result 的普通错误转成 AppError
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(format!("io error: {e}"))
    }
}
