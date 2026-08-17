//! 推送凭证存储

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::push::PushToken;
use crate::error::AppError;

#[async_trait]
pub trait PushTokenStore: Send + Sync {
    /// 保存（覆盖）用户的某通道凭证
    async fn save(&self, user_id: &str, channel: &str, token: &str) -> Result<(), AppError>;

    /// 列出指定用户所有启用的推送凭证
    async fn list_by_user(&self, user_id: &str) -> Result<Vec<PushToken>, AppError>;

    /// 批量查询多个用户的推送凭证（用于主播开播时通知所有订阅者）
    async fn list_by_users(&self, user_ids: &[String]) -> Result<Vec<PushToken>, AppError>;

    /// 删除用户某通道凭证
    async fn delete(&self, user_id: &str, channel: &str) -> Result<(), AppError>;
}

pub struct PgPushTokenStore {
    pool: PgPool,
}

impl PgPushTokenStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PushTokenStore for PgPushTokenStore {
    async fn save(&self, user_id: &str, channel: &str, token: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"INSERT INTO user_push_tokens (user_id, channel, token)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, channel) DO UPDATE SET token = EXCLUDED.token,
                                                               enabled = TRUE"#,
        )
        .bind(user_id)
        .bind(channel)
        .bind(token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<PushToken>, AppError> {
        let rows = sqlx::query_as::<_, PushToken>(
            r#"SELECT user_id, channel, token, enabled
               FROM user_push_tokens
               WHERE user_id = $1 AND enabled = TRUE"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_by_users(&self, user_ids: &[String]) -> Result<Vec<PushToken>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, PushToken>(
            r#"SELECT user_id, channel, token, enabled
               FROM user_push_tokens
               WHERE user_id = ANY($1) AND enabled = TRUE"#,
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete(&self, user_id: &str, channel: &str) -> Result<(), AppError> {
        sqlx::query(r#"DELETE FROM user_push_tokens WHERE user_id = $1 AND channel = $2"#)
            .bind(user_id)
            .bind(channel)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
