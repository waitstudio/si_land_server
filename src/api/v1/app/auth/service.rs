//! 认证业务编排：校验验证码、查找/创建用户、签发 JWT

use chrono::Utc;

use crate::config::constants;
use crate::domain::user::User;
use crate::error::AppError;
use crate::services::jwt;
use crate::state::AppState;
use crate::utils::phone::is_valid_phone;
use crate::utils::time;

use super::dto::LoginResponse;

pub struct AuthService;

impl AuthService {
    /// 校验验证码并登录，返回登录结果（含 JWT）
    pub async fn login(
        state: &AppState,
        phone: &str,
        code: &str,
    ) -> Result<LoginResponse, AppError> {
        if !is_valid_phone(phone) {
            return Err(AppError::invalid_param("手机号格式不正确"));
        }

        let stored = state.code_store.get(phone).await?;
        match stored {
            None => Err(AppError::sms_code_invalid("验证码错误或未发送")),
            Some(sms_code) => {
                if sms_code.is_expired(Utc::now()) {
                    state.code_store.remove(phone).await?;
                    return Err(AppError::sms_code_expired("验证码已过期"));
                }
                if sms_code.code != code {
                    return Err(AppError::sms_code_invalid("验证码错误"));
                }
                state.code_store.remove(phone).await?;

                let user = match state.user_store.find_by_phone(phone).await? {
                    Some(u) => u,
                    None => state
                        .user_store
                        .create(phone, &state.config.sms.default_nickname, "")
                        .await?,
                };
                let _ = state.user_store.touch_login(&user.user_id).await;

                let expires_at =
                    time::now_ts() + state.config.jwt.expires_hours * 3600;
                let token = jwt::sign(
                    &user.user_id,
                    &state.config.jwt.secret,
                    state.config.jwt.expires_hours,
                )?;

                Ok(LoginResponse {
                    token,
                    token_type: constants::TOKEN_TYPE.to_string(),
                    expires_at,
                    user,
                })
            }
        }
    }

    /// 根据当前 token 解析出的 user_id 查询用户信息
    pub async fn me(state: &AppState, user_id: &str) -> Result<User, AppError> {
        state
            .user_store
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))
    }

    /// 修改昵称
    ///
    /// 校验：去除首尾空白后长度 2-20 字符。
    pub async fn update_nickname(
        state: &AppState,
        user_id: &str,
        nickname: &str,
    ) -> Result<User, AppError> {
        let trimmed = nickname.trim();
        let len = trimmed.chars().count();
        if !(2..=20).contains(&len) {
            return Err(AppError::invalid_param("昵称长度需为 2-20 位"));
        }
        state
            .user_store
            .update_nickname(user_id, trimmed)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))
    }
}
