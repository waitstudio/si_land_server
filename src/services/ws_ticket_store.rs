//! 一次性 WebSocket ticket 存储。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

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

pub struct PgWsTicketStore {
    pool: PgPool,
}

impl PgWsTicketStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WsTicketStore for PgWsTicketStore {
    async fn issue(
        &self,
        ticket: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO ws_tickets (ticket, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(ticket)
            .bind(user_id)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn consume(&self, ticket: &str) -> Result<Option<String>, AppError> {
        let user_id = sqlx::query_scalar::<_, String>(
            "DELETE FROM ws_tickets WHERE ticket = $1 AND expires_at > NOW() RETURNING user_id",
        )
        .bind(ticket)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user_id)
    }
}
