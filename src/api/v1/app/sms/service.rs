//! 短信业务编排：生成验证码、存储、发送

use chrono::{Duration, Utc};
use rand::Rng;

use crate::domain::sms::SmsCode;
use crate::error::AppError;
use crate::state::AppState;
use crate::utils::phone::is_valid_phone;

pub struct SmsService;

impl SmsService {
    /// 向指定手机号发送验证码，返回有效期（秒）
    pub async fn send_code(state: &AppState, phone: &str) -> Result<u64, AppError> {
        if !is_valid_phone(phone) {
            return Err(AppError::invalid_param("手机号格式不正确"));
        }

        if !state
            .code_store
            .try_acquire_send(phone, state.config.sms.max_sends_per_hour, 3600)
            .await?
        {
            return Err(AppError::rate_limit(
                "验证码发送次数已达上限，请一小时后再试",
            ));
        }

        // 重发冷却检查
        if let Some(existing) = state.code_store.get(phone).await? {
            let elapsed = Utc::now().signed_duration_since(existing.created_at);
            if elapsed.num_seconds() < state.config.sms.resend_cooldown as i64 {
                return Err(AppError::rate_limit(format!(
                    "请{}秒后再试",
                    state.config.sms.resend_cooldown as i64 - elapsed.num_seconds()
                )));
            }
        }

        let code = generate_code(state);
        let now = Utc::now();
        let expire_in = state.config.sms.code_expire_in;

        state.sms_provider.send(phone, &code).await?;

        let sms_code = SmsCode {
            phone: phone.to_string(),
            code,
            created_at: now,
            expire_at: now + Duration::seconds(expire_in as i64),
        };
        state.code_store.save(sms_code, expire_in).await?;

        Ok(expire_in)
    }
}

/// 生成验证码：配置了 `mock_fixed_code` 则用固定码，否则随机 N 位
fn generate_code(state: &AppState) -> String {
    if let Some(fixed) = &state.config.sms.mock_fixed_code {
        tracing::debug!("使用固定验证码（mock）");
        return fixed.clone();
    }
    let mut rng = rand::thread_rng();
    (0..state.config.sms.code_length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}
