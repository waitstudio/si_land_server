//! 数据库迁移（SQL 文件驱动）
//!
//! 迁移以 `migrations/` 目录下的 SQL 文件为权威来源，启动时按文件名
//! 顺序幂等执行（全部语句使用 IF NOT EXISTS / IF EXISTS / ON CONFLICT）。
//! 新增迁移 = 新增 SQL 文件 + 在下方 MIGRATIONS 清单中登记。

use sqlx::PgPool;

use crate::error::AppError;

/// 迁移文件清单：按序执行，文件名必须升序排列
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_init.sql",
        include_str!("../../../migrations/0001_init.sql"),
    ),
    (
        "0002_poll_scheduler.sql",
        include_str!("../../../migrations/0002_poll_scheduler.sql"),
    ),
    (
        "0003_streamer_wishes.sql",
        include_str!("../../../migrations/0003_streamer_wishes.sql"),
    ),
    (
        "0004_feedback.sql",
        include_str!("../../../migrations/0004_feedback.sql"),
    ),
];

/// 执行全部迁移（幂等）
pub async fn run(pool: &PgPool) -> Result<(), AppError> {
    for (name, sql) in MIGRATIONS {
        // raw_sql 走简单查询协议，支持单文件多条语句
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(format!("执行迁移 {name} 失败: {e}")))?;
        tracing::debug!(migration = name, "已执行");
    }
    tracing::info!("数据库迁移完成（{} 个文件）", MIGRATIONS.len());
    Ok(())
}
