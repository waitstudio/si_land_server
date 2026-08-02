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

        let code = generate_code(state);
        let now = Utc::now();
        let expire_in = state.config.sms_code_expire_in;

        // 先发送再存储：发送失败则不占用存储槽位，用户可直接重发
        state.sms_provider.send(phone, &code).await?;

        let sms_code = SmsCode {
            phone: phone.to_string(),
            code,
            created_at: now,
            expire_at: now + Duration::seconds(expire_in as i64),
        };
        state.code_store.save(sms_code).await?;

        Ok(expire_in)
    }
}

/// 生成验证码：配置了 `mock_fixed_code` 则用固定码，否则随机 6 位
fn generate_code(state: &AppState) -> String {
    if let Some(fixed) = &state.config.mock_fixed_code {
        tracing::debug!("使用固定验证码（mock）");
        return fixed.clone();
    }
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}
