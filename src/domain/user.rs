//! 用户领域模型

use serde::Serialize;
use sqlx::FromRow;

/// 用户
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub user_id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar: String,
}
