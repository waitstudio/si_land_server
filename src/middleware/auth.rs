//! 认证中间件
//!
//! 从 `Authorization: Bearer <token>` 提取并校验 JWT，
//! 将 `UserId` 注入到 request extension 供 handler 使用。
//! 校验失败返回 401。

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::AppError;
use crate::services::jwt::{extract_bearer, verify};
use crate::state::AppState;

/// 注入到 request extension 的当前用户 ID
#[derive(Debug, Clone)]
pub struct UserId(pub String);

/// 认证中间件：State<AppState> 由 axum 从 Router state 自动注入
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::new(crate::response::BizCode::Unauthorized, "缺少 Authorization 头")
        })?;

    let token = extract_bearer(auth_header).ok_or_else(|| {
        AppError::new(crate::response::BizCode::Unauthorized, "无效的 Authorization 格式")
    })?;

    let claims = verify(token, &state.config.jwt.secret)?;
    req.extensions_mut().insert(UserId(claims.sub));
    Ok(next.run(req).await)
}
