//! 推送凭证业务编排

use crate::error::AppError;
use crate::state::AppState;

pub struct PushService;

impl PushService {
    /// 保存（覆盖）当前用户的某通道推送凭证
    pub async fn save_token(
        state: &AppState,
        user_id: &str,
        channel: &str,
        token: &str,
    ) -> Result<(), AppError> {
        validate_channel(channel)?;
        if token.trim().is_empty() {
            return Err(AppError::invalid_param("token 不能为空"));
        }
        state
            .push_token_store
            .save(user_id, channel, token)
            .await
    }

    /// 列出当前用户所有推送凭证
    pub async fn list_tokens(
        state: &AppState,
        user_id: &str,
    ) -> Result<Vec<crate::domain::push::PushToken>, AppError> {
        state.push_token_store.list_by_user(user_id).await
    }

    /// 删除当前用户某通道推送凭证
    pub async fn delete_token(
        state: &AppState,
        user_id: &str,
        channel: &str,
    ) -> Result<(), AppError> {
        validate_channel(channel)?;
        state.push_token_store.delete(user_id, channel).await
    }
}

/// 白名单校验通道名，防止注入未实现的通道
fn validate_channel(channel: &str) -> Result<(), AppError> {
    match channel {
        "bark" | "apns" => Ok(()),
        _ => Err(AppError::invalid_param(format!(
            "不支持的推送通道: {channel}"
        ))),
    }
}
