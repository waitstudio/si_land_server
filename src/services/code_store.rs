//! 验证码存储
//!
//! `CodeStore` 抽象验证码的存取，mock 阶段用内存实现，后续可替换为 Redis 实现。

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::domain::sms::SmsCode;
use crate::error::AppError;

/// 验证码存储
#[async_trait]
pub trait CodeStore: Send + Sync {
    /// 保存（覆盖）某手机号的验证码
    async fn save(&self, code: SmsCode) -> Result<(), AppError>;
    /// 读取某手机号的验证码
    async fn get(&self, phone: &str) -> Result<Option<SmsCode>, AppError>;
    /// 删除某手机号的验证码（验证成功 / 过期后调用）
    async fn remove(&self, phone: &str) -> Result<(), AppError>;
}

/// 内存验证码存储，mock 阶段使用
pub struct InMemoryCodeStore {
    inner: Mutex<HashMap<String, SmsCode>>,
}

impl InMemoryCodeStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCodeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CodeStore for InMemoryCodeStore {
    async fn save(&self, code: SmsCode) -> Result<(), AppError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| AppError::internal(format!("code store poisoned: {e}")))?;
        inner.insert(code.phone.clone(), code);
        Ok(())
    }

    async fn get(&self, phone: &str) -> Result<Option<SmsCode>, AppError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| AppError::internal(format!("code store poisoned: {e}")))?;
        Ok(inner.get(phone).cloned())
    }

    async fn remove(&self, phone: &str) -> Result<(), AppError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| AppError::internal(format!("code store poisoned: {e}")))?;
        inner.remove(phone);
        Ok(())
    }
}
