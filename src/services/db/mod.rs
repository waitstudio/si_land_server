//! 数据库连接池与迁移

pub mod migrations;
pub mod sql;

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::error::AppError;

/// 初始化 PostgreSQL 连接池
pub async fn init_pool(database_url: &str, max_connections: u32) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .map_err(|e| AppError::internal(format!("数据库连接失败: {e}")))
}
