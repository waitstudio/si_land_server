//! 统一错误类型，实现 IntoResponse 自动转成统一响应体

use std::fmt;

use axum::{
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

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.msg)
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn new(code: BizCode, msg: impl Into<String>) -> Self {
        Self { code, msg: msg.into() }
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

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(BizCode::NotFound, msg)
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(BizCode::Conflict, msg)
    }

    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::new(BizCode::RateLimit, msg)
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(BizCode::Unauthorized, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(BizCode::InternalError, msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.code.http_status();
        let body = ApiResponse::<serde_json::Value>::error(self.code, self.msg);
        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(format!("数据库错误: {e}"))
    }
}
