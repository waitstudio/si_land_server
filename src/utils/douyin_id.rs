//! 抖音号校验

use crate::error::AppError;

/// 校验抖音号格式
///
/// 规则：2-20 位，允许字母/数字/下划线/减号/中文，
/// 不允许 URL 特征字符（/ : ? # 空格 & =），避免误传链接。
pub fn validate(id: &str) -> Result<(), AppError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_param("抖音号不能为空"));
    }
    let len = trimmed.chars().count();
    if !(2..=20).contains(&len) {
        return Err(AppError::invalid_param("抖音号长度需为 2-20 位"));
    }
    if trimmed
        .chars()
        .any(|c| matches!(c, '/' | ':' | '?' | '#' | ' ' | '&' | '='))
    {
        return Err(AppError::invalid_param(
            "请输入抖音号，不要粘贴链接或分享口令",
        ));
    }
    Ok(())
}
