//! 短信发送通道
//!
//! `SmsProvider` 抽象短信发送能力，便于后续接入阿里云 / 腾讯云等真实通道。

use async_trait::async_trait;

use crate::error::AppError;

/// 短信通道
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// 发送验证码到指定手机号
    async fn send(&self, phone: &str, code: &str) -> Result<(), AppError>;
}

/// Mock 短信通道：不真正发送，仅在日志输出验证码
pub struct MockSmsProvider;

#[async_trait]
impl SmsProvider for MockSmsProvider {
    async fn send(&self, phone: &str, code: &str) -> Result<(), AppError> {
        tracing::info!(phone = %phone, code = %code, "短信验证码已发送（mock）");
        Ok(())
    }
}
