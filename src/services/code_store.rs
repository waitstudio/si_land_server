//! Redis 验证码存储与手机号维度限流。

use async_trait::async_trait;
use redis::AsyncCommands;

use crate::domain::sms::SmsCode;
use crate::error::AppError;

#[async_trait]
pub trait CodeStore: Send + Sync {
    async fn save(&self, code: SmsCode, ttl_secs: u64) -> Result<(), AppError>;
    async fn get(&self, phone: &str) -> Result<Option<SmsCode>, AppError>;
    async fn remove(&self, phone: &str) -> Result<(), AppError>;
    async fn try_acquire_send(
        &self,
        phone: &str,
        max_requests: i32,
        window_secs: i64,
    ) -> Result<bool, AppError>;
    async fn increment_failed_attempt(&self, phone: &str, ttl_secs: i64) -> Result<i32, AppError>;
}

pub struct RedisCodeStore {
    client: redis::Client,
}

impl RedisCodeStore {
    pub fn new(url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(url)
            .map_err(|error| AppError::internal(format!("Redis 配置错误: {error}")))?;
        Ok(Self { client })
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, AppError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::internal(format!("Redis 连接失败: {error}")))
    }

    fn code_key(phone: &str) -> String {
        format!("sms:code:{phone}")
    }

    fn attempt_key(phone: &str) -> String {
        format!("sms:attempt:{phone}")
    }

    fn rate_key(phone: &str) -> String {
        format!("sms:send-rate:{phone}")
    }
}

#[async_trait]
impl CodeStore for RedisCodeStore {
    async fn save(&self, code: SmsCode, ttl_secs: u64) -> Result<(), AppError> {
        let payload = serde_json::to_string(&code)
            .map_err(|error| AppError::internal(format!("验证码序列化失败: {error}")))?;
        let mut connection = self.connection().await?;
        let _: () = connection
            .set_ex(Self::code_key(&code.phone), payload, ttl_secs)
            .await
            .map_err(|error| AppError::internal(format!("Redis 写入验证码失败: {error}")))?;
        let _: () = connection
            .del(Self::attempt_key(&code.phone))
            .await
            .map_err(|error| {
                AppError::internal(format!("Redis 清理验证码尝试次数失败: {error}"))
            })?;
        Ok(())
    }

    async fn get(&self, phone: &str) -> Result<Option<SmsCode>, AppError> {
        let mut connection = self.connection().await?;
        let payload: Option<String> = connection
            .get(Self::code_key(phone))
            .await
            .map_err(|error| AppError::internal(format!("Redis 读取验证码失败: {error}")))?;
        payload
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| AppError::internal(format!("验证码数据损坏: {error}")))
            })
            .transpose()
    }

    async fn remove(&self, phone: &str) -> Result<(), AppError> {
        let mut connection = self.connection().await?;
        let _: () = connection
            .del(Self::code_key(phone))
            .await
            .map_err(|error| AppError::internal(format!("Redis 删除验证码失败: {error}")))?;
        let _: () = connection
            .del(Self::attempt_key(phone))
            .await
            .map_err(|error| AppError::internal(format!("Redis 删除验证码失败: {error}")))?;
        Ok(())
    }

    async fn try_acquire_send(
        &self,
        phone: &str,
        max_requests: i32,
        window_secs: i64,
    ) -> Result<bool, AppError> {
        let mut connection = self.connection().await?;
        let key = Self::rate_key(phone);
        let count: i32 = connection
            .incr(&key, 1)
            .await
            .map_err(|error| AppError::internal(format!("Redis 发送限流失败: {error}")))?;
        if count == 1 {
            let _: bool = connection
                .expire(&key, window_secs)
                .await
                .map_err(|error| {
                    AppError::internal(format!("Redis 设置发送限流过期失败: {error}"))
                })?;
        }
        Ok(count <= max_requests)
    }

    async fn increment_failed_attempt(&self, phone: &str, ttl_secs: i64) -> Result<i32, AppError> {
        let mut connection = self.connection().await?;
        let key = Self::attempt_key(phone);
        let attempts: i32 = connection
            .incr(&key, 1)
            .await
            .map_err(|error| AppError::internal(format!("Redis 验证码尝试限流失败: {error}")))?;
        if attempts == 1 {
            let _: bool = connection.expire(&key, ttl_secs).await.map_err(|error| {
                AppError::internal(format!("Redis 设置尝试限流过期失败: {error}"))
            })?;
        }
        Ok(attempts)
    }
}
