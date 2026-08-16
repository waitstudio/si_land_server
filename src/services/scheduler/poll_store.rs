//! 轮询任务存储：trait 抽象 + PostgreSQL 实现

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::scheduler::PollTask;
use crate::error::AppError;
use crate::utils::time;

/// 轮询任务存储抽象
#[async_trait]
pub trait PollStore: Send + Sync {
    /// 拉取到期任务，使用 FOR UPDATE SKIP LOCKED 避免多实例重复争抢
    ///
    /// 返回的任务记录已被当前事务锁定，调用方需在事务内更新 next_poll_at，
    /// 否则锁会在事务结束时自动释放。
    async fn fetch_due(
        &self,
        now: i64,
        limit: i64,
        lease_secs: i64,
    ) -> Result<Vec<PollTask>, AppError>;

    /// 原子地推进 next_poll_at 并更新 last_status / fail_count
    ///
    /// 通过 CAS 思想：将 next_poll_at 设为新的下次调度时间，
    /// 防止同一任务被重复调度。
    async fn schedule_next(
        &self,
        streamer_id: &str,
        next_poll_at: i64,
        last_status: i16,
        fail_count: i32,
    ) -> Result<(), AppError>;

    /// 主播订阅时自动创建轮询任务（幂等）
    async fn ensure_task(&self, streamer_id: &str) -> Result<(), AppError>;

    /// 设置轮询启停
    async fn set_enabled(&self, streamer_id: &str, enabled: bool) -> Result<(), AppError>;

    /// 查询订阅某主播的所有用户 ID（用于推送通知）
    async fn list_subscribers(&self, streamer_id: &str) -> Result<Vec<String>, AppError>;
}

pub struct PgPollStore {
    pool: PgPool,
}

impl PgPollStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PollStore for PgPollStore {
    async fn fetch_due(
        &self,
        now: i64,
        limit: i64,
        lease_secs: i64,
    ) -> Result<Vec<PollTask>, AppError> {
        let mut tx = self.pool.begin().await?;

        // FOR UPDATE SKIP LOCKED：多实例部署时避免任务被重复抓取
        let tasks = sqlx::query_as::<_, PollTask>(
            r#"SELECT streamer_id, poll_enabled, next_poll_at, last_status,
                      last_poll_at, fail_count
               FROM streamer_poll_tasks
               WHERE poll_enabled = TRUE
                 AND next_poll_at <= $1
                 AND (lease_until IS NULL OR lease_until <= $1)
               ORDER BY next_poll_at
               LIMIT $2
               FOR UPDATE SKIP LOCKED"#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        // 为已领取任务写入有限期 lease。实例崩溃后 lease 到期即可被其他实例恢复。
        let ids: Vec<&str> = tasks.iter().map(|t| t.streamer_id.as_str()).collect();
        if !ids.is_empty() {
            sqlx::query(
                r#"UPDATE streamer_poll_tasks
                   SET lease_until = $1
                   WHERE streamer_id = ANY($2)"#,
            )
            .bind(now + lease_secs)
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(tasks)
    }

    async fn schedule_next(
        &self,
        streamer_id: &str,
        next_poll_at: i64,
        last_status: i16,
        fail_count: i32,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE streamer_poll_tasks
               SET next_poll_at = $1,
                   last_status = $2,
                   fail_count = $3,
                   last_poll_at = $4,
                   lease_until = NULL
               WHERE streamer_id = $5"#,
        )
        .bind(next_poll_at)
        .bind(last_status)
        .bind(fail_count)
        .bind(time::now_ts())
        .bind(streamer_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn ensure_task(&self, streamer_id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"INSERT INTO streamer_poll_tasks (streamer_id, next_poll_at)
               VALUES ($1, $2)
               ON CONFLICT (streamer_id) DO NOTHING"#,
        )
        .bind(streamer_id)
        .bind(time::now_ts())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_enabled(&self, streamer_id: &str, enabled: bool) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE streamer_poll_tasks
               SET poll_enabled = $1
               WHERE streamer_id = $2"#,
        )
        .bind(enabled)
        .bind(streamer_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_subscribers(&self, streamer_id: &str) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_scalar::<_, String>(
            r#"SELECT user_id FROM subscriptions WHERE streamer_id = $1"#,
        )
        .bind(streamer_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
