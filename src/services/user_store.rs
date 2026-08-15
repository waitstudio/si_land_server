//! 用户存储

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::user::User;
use crate::error::AppError;
use crate::utils::id;

/// 用户存储抽象
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError>;
    async fn create(&self, phone: &str, nickname: &str, avatar: &str) -> Result<User, AppError>;
    async fn touch_login(&self, user_id: &str) -> Result<(), AppError>;
}

/// PostgreSQL 实现
pub struct PgUserStore {
    pool: PgPool,
}

impl PgUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserStore for PgUserStore {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, User>(
            r#"SELECT user_id, phone, nickname, avatar FROM users WHERE phone = $1"#,
        )
        .bind(phone)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, User>(
            r#"SELECT user_id, phone, nickname, avatar FROM users WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create(&self, phone: &str, nickname: &str, avatar: &str) -> Result<User, AppError> {
        let user_id = id::gen_user_id();
        sqlx::query(
            r#"INSERT INTO users (user_id, phone, nickname, avatar)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(&user_id)
        .bind(phone)
        .bind(nickname)
        .bind(avatar)
        .execute(&self.pool)
        .await?;

        Ok(User {
            user_id,
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: avatar.to_string(),
        })
    }

    async fn touch_login(&self, user_id: &str) -> Result<(), AppError> {
        sqlx::query(r#"UPDATE users SET last_login_at = NOW() WHERE user_id = $1"#)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
