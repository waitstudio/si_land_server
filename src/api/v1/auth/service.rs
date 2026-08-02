//! 认证业务编排：校验验证码、签发 token

use chrono::Utc;

use crate::domain::user::User;
use crate::error::AppError;
use crate::state::AppState;
use crate::utils::phone::is_valid_phone;

use super::dto::LoginResponse;

pub struct AuthService;

impl AuthService {
    /// 校验验证码并登录，返回登录结果
    pub async fn login(
        state: &AppState,
        phone: &str,
        code: &str,
    ) -> Result<LoginResponse, AppError> {
        if !is_valid_phone(phone) {
            return Err(AppError::invalid_param("手机号格式不正确"));
        }

        // 校验验证码
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
                // 验证通过，移除验证码（一次性）
                state.code_store.remove(phone).await?;

                // mock：生成 token 与用户信息
                let user = mock_user(phone);
                let expires_at = Utc::now().timestamp() + state.config.jwt_expires_hours * 3600;
                Ok(LoginResponse {
                    token: format!("mock-token-{}", user.user_id),
                    token_type: "Bearer".to_string(),
                    expires_at,
                    user,
                })
            }
        }
    }
}

/// mock 用户：手机号后 4 位作为 user_id
fn mock_user(phone: &str) -> User {
    let user_id = format!("u_{}", &phone[phone.len().saturating_sub(4)..]);
    User {
        user_id,
        phone: phone.to_string(),
        nickname: "硅基星球用户".to_string(),
        avatar: "".to_string(),
    }
}
