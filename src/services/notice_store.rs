//! 开播通知存储
//!
//! [NoticeStore] 抽象通知的存取，[PgNoticeStore] 为 PostgreSQL 实现。
//! 轮询调度器在主播开播时调用 [NoticeStore::create] 落库一条通知，
//! 随后由推送通道下发；接口层提供分页查询 / 标记已读 / 删除能力。

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::notice::LiveNotice;
use crate::error::AppError;

/// 通知存储抽象
#[async_trait]
pub trait NoticeStore: Send + Sync {
    /// 创建一条通知（用于轮询发现"未播→开播"时落库）
    async fn create(
        &self,
        user_id: &str,
        streamer_id: &str,
        streamer_nickname: &str,
        title: &str,
        body: &str,
        live_started_at: Option<i64>,
        created_at: i64,
    ) -> Result<LiveNotice, AppError>;

    /// 分页查询某用户的通知列表（按 created_at 倒序）
    async fn list_page(
        &self,
        user_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<LiveNotice>, i64), AppError>;

    /// 统计某用户的未读通知数
    async fn unread_count(&self, user_id: &str) -> Result<i64, AppError>;

    /// 标记单条通知为已读（需校验归属 user_id，避免越权操作他人通知）
    /// 返回是否实际更新（通知不存在或不属于该用户时返回 false）
    async fn mark_read(&self, user_id: &str, notice_id: &str) -> Result<bool, AppError>;

    /// 标记某用户全部通知为已读，返回受影响行数
    async fn mark_all_read(&self, user_id: &str) -> Result<i64, AppError>;

    /// 删除单条通知（需校验归属 user_id），返回是否删除
    async fn delete(&self, user_id: &str, notice_id: &str) -> Result<bool, AppError>;
}

pub struct PgNoticeStore {
    pool: PgPool,
}

impl PgNoticeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NoticeStore for PgNoticeStore {
    async fn create(
        &self,
        user_id: &str,
        streamer_id: &str,
        streamer_nickname: &str,
        title: &str,
        body: &str,
        live_started_at: Option<i64>,
        created_at: i64,
    ) -> Result<LiveNotice, AppError> {
        // 主键由调用方传入的 id 生成器产出，这里直接绑定
        let id = crate::utils::id::gen_notice_id();
        let notice = sqlx::query_as::<_, LiveNotice>(
            r#"INSERT INTO live_notices
                   (id, user_id, streamer_id, streamer_nickname, title, body,
                    live_started_at, created_at, read)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE)
               RETURNING id, user_id, streamer_id, streamer_nickname, title, body,
                         live_started_at, created_at, read"#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(streamer_id)
        .bind(streamer_nickname)
        .bind(title)
        .bind(body)
        .bind(live_started_at)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(notice)
    }

    async fn list_page(
        &self,
        user_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<LiveNotice>, i64), AppError> {
        let offset = (page - 1).max(0) * page_size;
        // LEFT JOIN streamers 获取主播头像（不入库，实时关联）
        let items = sqlx::query_as::<_, LiveNotice>(
            r#"SELECT n.id, n.user_id, n.streamer_id, n.streamer_nickname,
                      n.title, n.body, n.live_started_at, n.created_at, n.read,
                      s.avatar AS avatar
               FROM live_notices n
               LEFT JOIN streamers s ON n.streamer_id = s.id
               WHERE n.user_id = $1
               ORDER BY n.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM live_notices WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((items, total))
    }

    async fn unread_count(&self, user_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM live_notices WHERE user_id = $1 AND read = FALSE"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    async fn mark_read(&self, user_id: &str, notice_id: &str) -> Result<bool, AppError> {
        let rows = sqlx::query(
            r#"UPDATE live_notices SET read = TRUE
               WHERE id = $1 AND user_id = $2 AND read = FALSE"#,
        )
        .bind(notice_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(rows.rows_affected() > 0)
    }

    async fn mark_all_read(&self, user_id: &str) -> Result<i64, AppError> {
        let rows = sqlx::query(
            r#"UPDATE live_notices SET read = TRUE
               WHERE user_id = $1 AND read = FALSE"#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(rows.rows_affected() as i64)
    }

    async fn delete(&self, user_id: &str, notice_id: &str) -> Result<bool, AppError> {
        let rows = sqlx::query(
            r#"DELETE FROM live_notices WHERE id = $1 AND user_id = $2"#,
        )
        .bind(notice_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(rows.rows_affected() > 0)
    }
}
