//! 问题反馈存储
//!
//! [FeedbackStore] 抽象反馈的存取，[PgFeedbackStore] 为 PostgreSQL 实现。
//! App 端"我的-问题反馈"入口提交 BUG / 建议，运营按表跟进。

use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::AppError;
use crate::utils::id;

/// 问题反馈存储抽象
#[async_trait]
pub trait FeedbackStore: Send + Sync {
    /// 落库一条反馈，返回反馈 ID
    async fn create(&self, user_id: &str, content: &str) -> Result<String, AppError>;
}

pub struct PgFeedbackStore {
    pool: PgPool,
}

impl PgFeedbackStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FeedbackStore for PgFeedbackStore {
    async fn create(&self, user_id: &str, content: &str) -> Result<String, AppError> {
        let id = id::gen_feedback_id();
        sqlx::query(
            r#"INSERT INTO feedbacks (id, user_id, content)
               VALUES ($1, $2, $3)"#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(content)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("落库反馈失败: {e}")))?;
        Ok(id)
    }
}
