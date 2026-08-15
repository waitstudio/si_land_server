//! 主播与订阅存储
//!
//! `SubscriptionStore` 抽象主播信息与订阅关系的存取。
//! `PgSubscriptionStore` 为 PostgreSQL 实现，数据持久化。

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::streamer::Streamer;
use crate::domain::subscription::SubscriptionItem;
use crate::error::AppError;
use crate::services::db::sql::STREAMER_COLUMNS;

/// 订阅与主播存储抽象
#[async_trait]
pub trait SubscriptionStore: Send + Sync {
    /// 保存（覆盖）主播信息
    async fn save_streamer(&self, streamer: Streamer) -> Result<(), AppError>;
    /// 按主键查询主播
    async fn get_streamer(&self, id: &str) -> Result<Option<Streamer>, AppError>;
    /// 按 sec_uid 查询主播
    async fn find_by_sec_uid(&self, sec_uid: &str) -> Result<Option<Streamer>, AppError>;
    /// 建立订阅关系，返回是否为新增
    async fn subscribe(
        &self,
        user_id: &str,
        streamer_id: &str,
        subscribed_at: i64,
    ) -> Result<bool, AppError>;
    /// 取消订阅
    async fn unsubscribe(&self, user_id: &str, streamer_id: &str) -> Result<(), AppError>;
    /// 列出用户订阅的主播（含订阅时间）
    async fn list_subscriptions(&self, user_id: &str) -> Result<Vec<SubscriptionItem>, AppError>;
    /// 列出热门主播
    async fn list_popular(&self, limit: i64) -> Result<Vec<Streamer>, AppError>;
    /// 主播人气值 +1
    async fn inc_popularity(&self, streamer_id: &str) -> Result<(), AppError>;
    /// 设置主播开播状态并同步昵称/头像，返回更新后的主播
    ///
    /// `nickname` / `avatar` 传 `None` 时保持原值不变，
    /// 用于轮询时把抖音接口返回的最新资料同步回 streamers 表。
    async fn set_live(
        &self,
        streamer_id: &str,
        live: bool,
        started_at: Option<i64>,
        nickname: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<Option<Streamer>, AppError>;
}

/// PostgreSQL 实现
pub struct PgSubscriptionStore {
    pool: PgPool,
}

impl PgSubscriptionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SubscriptionStore for PgSubscriptionStore {
    async fn save_streamer(&self, streamer: Streamer) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO streamers
                (id, sec_uid, douyin_id, nickname, avatar, live, live_started_at, popularity)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (sec_uid) DO UPDATE SET
                douyin_id       = EXCLUDED.douyin_id,
                nickname        = EXCLUDED.nickname,
                avatar          = EXCLUDED.avatar,
                live            = EXCLUDED.live,
                live_started_at = EXCLUDED.live_started_at
            "#,
        )
        .bind(&streamer.id)
        .bind(&streamer.sec_uid)
        .bind(&streamer.douyin_id)
        .bind(&streamer.nickname)
        .bind(&streamer.avatar)
        .bind(streamer.live)
        .bind(streamer.live_started_at)
        .bind(streamer.popularity)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_streamer(&self, id: &str) -> Result<Option<Streamer>, AppError> {
        let sql = format!("SELECT {STREAMER_COLUMNS} FROM streamers WHERE id = $1");
        let row = sqlx::query_as::<_, Streamer>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn find_by_sec_uid(&self, sec_uid: &str) -> Result<Option<Streamer>, AppError> {
        let sql = format!("SELECT {STREAMER_COLUMNS} FROM streamers WHERE sec_uid = $1");
        let row = sqlx::query_as::<_, Streamer>(&sql)
            .bind(sec_uid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn subscribe(
        &self,
        user_id: &str,
        streamer_id: &str,
        subscribed_at: i64,
    ) -> Result<bool, AppError> {
        let rows = sqlx::query(
            r#"INSERT INTO subscriptions (user_id, streamer_id, subscribed_at)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, streamer_id) DO NOTHING"#,
        )
        .bind(user_id)
        .bind(streamer_id)
        .bind(subscribed_at)
        .execute(&self.pool)
        .await?;
        Ok(rows.rows_affected() > 0)
    }

    async fn unsubscribe(&self, user_id: &str, streamer_id: &str) -> Result<(), AppError> {
        sqlx::query(r#"DELETE FROM subscriptions WHERE user_id = $1 AND streamer_id = $2"#)
            .bind(user_id)
            .bind(streamer_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_subscriptions(&self, user_id: &str) -> Result<Vec<SubscriptionItem>, AppError> {
        // 正在直播的主播排最前，未直播的放后面；两组内部均按订阅时间倒序
        let sql = r#"SELECT s.id, s.sec_uid, s.douyin_id, s.nickname, s.avatar, s.live,
                            s.live_started_at, s.popularity, sub.subscribed_at
                     FROM streamers s
                     JOIN subscriptions sub ON sub.streamer_id = s.id
                     WHERE sub.user_id = $1
                     ORDER BY s.live DESC, sub.subscribed_at DESC"#;
        let list = sqlx::query_as::<_, SubscriptionItem>(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(list)
    }

    async fn list_popular(&self, limit: i64) -> Result<Vec<Streamer>, AppError> {
        // 按人气降序返回全部主播（含 0 人气），同人气按最近更新优先
        let sql = format!(
            "SELECT {STREAMER_COLUMNS} FROM streamers \
             ORDER BY popularity DESC, updated_at DESC LIMIT $1"
        );
        let list = sqlx::query_as::<_, Streamer>(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        Ok(list)
    }

    async fn inc_popularity(&self, streamer_id: &str) -> Result<(), AppError> {
        sqlx::query(r#"UPDATE streamers SET popularity = popularity + 1 WHERE id = $1"#)
            .bind(streamer_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn set_live(
        &self,
        streamer_id: &str,
        live: bool,
        started_at: Option<i64>,
        nickname: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<Option<Streamer>, AppError> {
        // COALESCE：传入 None 时保持原值，传入 Some 时覆盖
        let sql = format!(
            "UPDATE streamers SET live = $1, live_started_at = $2, \
                                 nickname = COALESCE($3, nickname), \
                                 avatar = COALESCE($4, avatar) \
             WHERE id = $5 RETURNING {STREAMER_COLUMNS}"
        );
        let row = sqlx::query_as::<_, Streamer>(&sql)
            .bind(live)
            .bind(started_at)
            .bind(nickname)
            .bind(avatar)
            .bind(streamer_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }
}
