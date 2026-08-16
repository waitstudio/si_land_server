//! 通知 Outbox：将通知创建与外部下发解耦，支持失败重试和实例崩溃恢复。

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use crate::error::AppError;
use crate::utils::id::gen_outbox_id;

#[derive(Debug, Clone, FromRow)]
pub struct OutboxEvent {
    pub id: String,
    pub user_id: String,
    pub payload: Value,
    pub attempts: i32,
}

#[async_trait]
pub trait NotificationOutbox: Send + Sync {
    async fn enqueue(&self, user_id: &str, payload: Value) -> Result<(), AppError>;
    async fn claim_due(
        &self,
        now: i64,
        limit: i64,
        lease_secs: i64,
    ) -> Result<Vec<OutboxEvent>, AppError>;
    async fn complete(&self, id: &str) -> Result<(), AppError>;
    async fn retry(&self, id: &str, next_attempt_at: i64, error: &str) -> Result<(), AppError>;
}

pub struct PgNotificationOutbox {
    pool: PgPool,
}

impl PgNotificationOutbox {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationOutbox for PgNotificationOutbox {
    async fn enqueue(&self, user_id: &str, payload: Value) -> Result<(), AppError> {
        sqlx::query(
            r#"INSERT INTO notification_outbox (id, user_id, payload, next_attempt_at)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(gen_outbox_id())
        .bind(user_id)
        .bind(payload)
        .bind(crate::utils::time::now_ts())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn claim_due(
        &self,
        now: i64,
        limit: i64,
        lease_secs: i64,
    ) -> Result<Vec<OutboxEvent>, AppError> {
        let mut tx = self.pool.begin().await?;
        let events = sqlx::query_as::<_, OutboxEvent>(
            r#"SELECT id, user_id, payload, attempts
               FROM notification_outbox
               WHERE next_attempt_at <= $1 AND (lease_until IS NULL OR lease_until <= $1)
               ORDER BY next_attempt_at, id
               LIMIT $2
               FOR UPDATE SKIP LOCKED"#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        let ids: Vec<&str> = events.iter().map(|event| event.id.as_str()).collect();
        if !ids.is_empty() {
            sqlx::query(
                r#"UPDATE notification_outbox
                   SET lease_until = $1, attempts = attempts + 1
                   WHERE id = ANY($2)"#,
            )
            .bind(now + lease_secs)
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(events)
    }

    async fn complete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM notification_outbox WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn retry(&self, id: &str, next_attempt_at: i64, error: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE notification_outbox
               SET lease_until = NULL, next_attempt_at = $1, last_error = $2
               WHERE id = $3"#,
        )
        .bind(next_attempt_at)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
