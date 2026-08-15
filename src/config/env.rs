//! 环境变量读取工具
//!
//! 区分必填（`required`）、可选（`optional`）、带默认值（`or` / `parse_or`）。

use std::env;

use crate::error::AppError;

/// 读取必填环境变量，缺失返回错误
pub fn required(key: &str) -> Result<String, AppError> {
    env::var(key).map_err(|_| {
        AppError::internal(format!("缺少必填环境变量: {key}"))
    })
}

/// 读取环境变量，缺失返回默认值
pub fn or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 读取可选环境变量，空字符串视为未设置
pub fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 解析环境变量，失败或缺失返回默认值
pub fn parse_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
