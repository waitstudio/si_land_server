//! 一次性 WebSocket ticket 存储（Redis）。
//!
//! ticket 生命周期极短（默认 60s）且只消费一次：
//! - 写入时以剩余有效期为 TTL，到期自动清理，无需建表和后台任务
//! - 消费用 GETDEL 原子完成，保证同一 ticket 只能握手成功一次

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;

use crate::error::AppError;

#[async_trait]
pub trait WsTicketStore: Send + Sync {
    async fn issue(
        &self,
        ticket: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn consume(&self, ticket: &str) -> Result<Option<String>, AppError>;
}

pub struct RedisWsTicketStore {
    client: redis::Client,
}

impl RedisWsTicketStore {
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

    fn ticket_key(ticket: &str) -> String {
        format!("ws:ticket:{ticket}")
    }
}

#[async_trait]
impl WsTicketStore for RedisWsTicketStore {
    async fn issue(
        &self,
        ticket: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        // 剩余有效期作为 TTL，到期由 Redis 自动删除
        let ttl = (expires_at - Utc::now()).num_seconds().max(1);
        let mut connection = self.connection().await?;
        let _: () = connection
            .set_ex(Self::ticket_key(ticket), user_id, ttl as u64)
            .await
            .map_err(|error| AppError::internal(format!("Redis 写入 WS ticket 失败: {error}")))?;
        Ok(())
    }

    async fn consume(&self, ticket: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection().await?;
        // GETDEL：原子读取并删除，一次性消费语义
        let user_id: Option<String> = redis::cmd("GETDEL")
            .arg(Self::ticket_key(ticket))
            .query_async(&mut connection)
            .await
            .map_err(|error| AppError::internal(format!("Redis 消费 WS ticket 失败: {error}")))?;
        Ok(user_id)
    }
}
