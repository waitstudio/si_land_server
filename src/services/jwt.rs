//! JWT 签发与校验
//!
//! Claims 中只放 `sub`（user_id）与 `exp`，敏感信息不放 token。
//! 签名算法 HS256，密钥从 `AppConfig.jwt_secret` 读取。

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// user_id
    pub sub: String,
    /// 过期时间戳（秒）
    pub exp: i64,
}

/// 签发 token
pub fn sign(user_id: &str, secret: &str, expires_hours: i64) -> Result<String, AppError> {
    let exp = (Utc::now() + Duration::hours(expires_hours)).timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("签发 token 失败: {e}")))
}

/// 校验 token，返回 Claims
pub fn verify(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::new(crate::response::BizCode::Unauthorized, format!("token 无效: {e}")))
}

/// 从 `Authorization: Bearer <token>` 提取 token
pub fn extract_bearer(header_value: &str) -> Option<&str> {
    let trimmed = header_value.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(|t| t.trim())
}
