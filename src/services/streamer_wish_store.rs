//! 主播想看意愿存储
//!
//! 收集用户想看但尚未收录的主播，按抖音号去重，记录想看计数。
//! 运营根据计数决定是否将该主播加入 streamers 表。

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::wish::StreamerWish;
use crate::error::AppError;

/// 想看意愿存储抽象
#[async_trait]
pub trait StreamerWishStore: Send + Sync {
    /// 提交想看意愿（按 douyin_id 去重，已存在则计数 +1），返回当前想看人数
    async fn upsert_wish(&self, douyin_id: &str) -> Result<i64, AppError>;

    /// 想看意愿列表（按想看人数降序），供运营查询
    async fn list_wishes(&self, limit: i64) -> Result<Vec<StreamerWish>, AppError>;
}

/// PostgreSQL 实现
pub struct PgStreamerWishStore {
    pool: PgPool,
}

impl PgStreamerWishStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StreamerWishStore for PgStreamerWishStore {
    async fn upsert_wish(&self, douyin_id: &str) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as(
            r#"INSERT INTO streamer_wishes (douyin_id, want_count)
               VALUES ($1, 1)
               ON CONFLICT (douyin_id) DO UPDATE
                   SET want_count = streamer_wishes.want_count + 1,
                       updated_at = NOW()
               RETURNING want_count"#,
        )
        .bind(douyin_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn list_wishes(&self, limit: i64) -> Result<Vec<StreamerWish>, AppError> {
        let list = sqlx::query_as::<_, StreamerWish>(
            r#"SELECT douyin_id, want_count, created_at, updated_at
               FROM streamer_wishes
               ORDER BY want_count DESC, updated_at DESC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }
}
