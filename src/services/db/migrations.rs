//! 建表迁移（幂等）
//!
//! 启动时自动执行 `CREATE TABLE IF NOT EXISTS`，不依赖外部 sqlx-cli，
//! 部署简单。SQL 同步维护在 `migrations/0001_init.sql` 便于审计。

use sqlx::PgPool;

use crate::error::AppError;

/// 执行建表迁移（幂等）
pub async fn run(pool: &PgPool) -> Result<(), AppError> {
    // users 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            user_id      VARCHAR(64) PRIMARY KEY,
            phone        VARCHAR(20) UNIQUE NOT NULL,
            nickname     VARCHAR(64) NOT NULL DEFAULT '',
            avatar       TEXT NOT NULL DEFAULT '',
            status       SMALLINT NOT NULL DEFAULT 1,
            created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_login_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 users 失败: {e}")))?;

    // streamers 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS streamers (
            id              VARCHAR(64) PRIMARY KEY,
            sec_uid         VARCHAR(128) UNIQUE NOT NULL,
            douyin_id       VARCHAR(64) NOT NULL DEFAULT '',
            nickname        VARCHAR(128) NOT NULL DEFAULT '',
            avatar          TEXT NOT NULL DEFAULT '',
            live            BOOLEAN NOT NULL DEFAULT FALSE,
            live_started_at BIGINT,
            popularity      BIGINT NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 streamers 失败: {e}")))?;

    // 兼容已存在表：补 popularity 字段
    sqlx::query(
        r#"ALTER TABLE streamers ADD COLUMN IF NOT EXISTS popularity BIGINT NOT NULL DEFAULT 0"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("添加 popularity 字段失败: {e}")))?;

    // popularity 索引（复合 updated_at 兜底排序）
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_streamers_popularity
           ON streamers (popularity DESC, updated_at DESC)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 popularity 索引失败: {e}")))?;

    // subscriptions 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS subscriptions (
            user_id        VARCHAR(64) NOT NULL,
            streamer_id    VARCHAR(64) NOT NULL,
            subscribed_at  BIGINT NOT NULL,
            PRIMARY KEY (user_id, streamer_id),
            FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
            FOREIGN KEY (streamer_id) REFERENCES streamers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 subscriptions 失败: {e}")))?;

    // subscriptions 索引：按 streamer_id 反查（级联删除用）
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_subscriptions_streamer ON subscriptions (streamer_id)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 subscriptions 索引失败: {e}")))?;

    // subscriptions 索引：按 user_id 查订阅列表 + 按订阅时间排序
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_subscriptions_user
           ON subscriptions (user_id, subscribed_at DESC)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 subscriptions user 索引失败: {e}")))?;

    // updated_at 触发器函数
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建触发器函数失败: {e}")))?;

    // 触发器：必须逐条执行（prepared statement 不允许多条命令）
    for table in ["users", "streamers"] {
        sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS trg_{table}_updated ON {table}"
        ))
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("删除 {table} 触发器失败: {e}")))?;

        sqlx::query(&format!(
            "CREATE TRIGGER trg_{table}_updated BEFORE UPDATE ON {table} \
             FOR EACH ROW EXECUTE FUNCTION touch_updated_at()"
        ))
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("创建 {table} 触发器失败: {e}")))?;
    }

    // 0002：轮询调度与推送通道
    run_0002_poll_scheduler(pool).await?;

    // 0003：昵称唯一约束
    run_0003_nickname_unique(pool).await?;

    // 0004：主播想看意愿收集表
    run_0004_streamer_wishes(pool).await?;

    // 0005：问题反馈收集表
    run_0005_feedback(pool).await?;

    // 0007：WebSocket 一次性 ticket
    run_0007_ws_tickets(pool).await?;

    // 0008：通知 Outbox
    run_0008_notification_outbox(pool).await?;

    tracing::info!("数据库迁移完成");
    Ok(())
}

/// 0002 迁移：轮询调度表与推送凭证表
async fn run_0002_poll_scheduler(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS streamer_poll_tasks (
            streamer_id    VARCHAR(64) PRIMARY KEY,
            poll_enabled   BOOLEAN     NOT NULL DEFAULT TRUE,
            next_poll_at   BIGINT      NOT NULL DEFAULT 0,
            last_status    SMALLINT    NOT NULL DEFAULT 0,
            last_poll_at   BIGINT,
            fail_count     INT         NOT NULL DEFAULT 0,
            CONSTRAINT fk_poll_streamer
                FOREIGN KEY (streamer_id) REFERENCES streamers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 streamer_poll_tasks 失败: {e}")))?;

    sqlx::query("ALTER TABLE streamer_poll_tasks ADD COLUMN IF NOT EXISTS lease_until BIGINT")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("添加 streamer_poll_tasks lease 失败: {e}")))?;

    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_poll_next
           ON streamer_poll_tasks (next_poll_at)
           WHERE poll_enabled = TRUE"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 idx_poll_next 失败: {e}")))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_push_tokens (
            user_id    VARCHAR(64) NOT NULL,
            channel    VARCHAR(16) NOT NULL,
            token      TEXT        NOT NULL,
            enabled    BOOLEAN     NOT NULL DEFAULT TRUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, channel),
            CONSTRAINT fk_push_user
                FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 user_push_tokens 失败: {e}")))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON user_push_tokens (user_id)"#)
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("创建 idx_push_tokens_user 失败: {e}")))?;

    // 开播通知表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS live_notices (
            id                 VARCHAR(64) PRIMARY KEY,
            user_id            VARCHAR(64) NOT NULL,
            streamer_id        VARCHAR(64) NOT NULL,
            streamer_nickname  VARCHAR(128) NOT NULL DEFAULT '',
            title              VARCHAR(128) NOT NULL DEFAULT '',
            body               TEXT NOT NULL DEFAULT '',
            live_started_at    BIGINT,
            created_at         BIGINT NOT NULL,
            read               BOOLEAN NOT NULL DEFAULT FALSE,
            CONSTRAINT fk_notice_user     FOREIGN KEY (user_id)
                REFERENCES users(user_id) ON DELETE CASCADE,
            CONSTRAINT fk_notice_streamer  FOREIGN KEY (streamer_id)
                REFERENCES streamers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 live_notices 失败: {e}")))?;

    // 分页查询主索引（按用户 + 时间倒序）
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_notices_user_time
           ON live_notices (user_id, created_at DESC)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 idx_notices_user_time 失败: {e}")))?;

    // 未读计数部分索引（只索引未读行，计数更快）
    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_notices_unread
           ON live_notices (user_id) WHERE read = FALSE"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 idx_notices_unread 失败: {e}")))?;

    // 回填：把已存在但未在 streamer_poll_tasks 中的主播全部纳入轮询
    // 幂等：ON CONFLICT DO NOTHING 跳过已有任务；新任务 next_poll_at=0 立即触发首次检测
    let backfilled = sqlx::query(
        r#"INSERT INTO streamer_poll_tasks (streamer_id, next_poll_at)
           SELECT id, 0 FROM streamers
           WHERE id NOT IN (SELECT streamer_id FROM streamer_poll_tasks)
           ON CONFLICT (streamer_id) DO NOTHING"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("回填 streamer_poll_tasks 失败: {e}")))?;
    if backfilled.rows_affected() > 0 {
        tracing::info!(
            "回填 {} 个主播到 streamer_poll_tasks",
            backfilled.rows_affected()
        );
    }

    Ok(())
}

/// 0003 迁移：昵称约束（已回退）
///
/// 移除昵称唯一索引，允许昵称重复。
async fn run_0003_nickname_unique(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(r#"DROP INDEX IF EXISTS idx_users_nickname_unique"#)
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("删除 nickname 唯一索引失败: {e}")))?;
    Ok(())
}

/// 0004 迁移：主播想看意愿收集表
///
/// 按 douyin_id 去重，记录想看计数，运营据此决定是否收录该主播。
async fn run_0004_streamer_wishes(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS streamer_wishes (
            douyin_id   VARCHAR(64) PRIMARY KEY,
            want_count  BIGINT NOT NULL DEFAULT 1,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 streamer_wishes 失败: {e}")))?;

    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_streamer_wishes_count
           ON streamer_wishes (want_count DESC)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 streamer_wishes 索引失败: {e}")))?;
    Ok(())
}

/// 0005 迁移：问题反馈收集表
///
/// 收集用户提交的 BUG、功能建议，运营据此跟进改进。
async fn run_0005_feedback(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS feedbacks (
            id         VARCHAR(64) PRIMARY KEY,
            user_id    VARCHAR(64) NOT NULL,
            content    VARCHAR(500) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT fk_feedback_user
                FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 feedbacks 失败: {e}")))?;

    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_feedbacks_user_time
           ON feedbacks (user_id, created_at DESC)"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 feedbacks 索引失败: {e}")))?;
    Ok(())
}

async fn run_0007_ws_tickets(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS ws_tickets (
            ticket      VARCHAR(64) PRIMARY KEY,
            user_id     VARCHAR(64) NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            expires_at  TIMESTAMPTZ NOT NULL
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 ws_tickets 失败: {e}")))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_ws_tickets_expiry ON ws_tickets (expires_at)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("创建 ws_tickets 索引失败: {e}")))?;
    Ok(())
}

async fn run_0008_notification_outbox(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS notification_outbox (
            id              VARCHAR(64) PRIMARY KEY,
            user_id         VARCHAR(64) NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            payload         JSONB NOT NULL,
            attempts        INT NOT NULL DEFAULT 0,
            next_attempt_at BIGINT NOT NULL,
            lease_until     BIGINT,
            last_error      TEXT
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("建表 notification_outbox 失败: {e}")))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_notification_outbox_due ON notification_outbox (next_attempt_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("创建 notification_outbox 索引失败: {e}")))?;
    Ok(())
}
