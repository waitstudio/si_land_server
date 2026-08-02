//! HTTP API 层
//!
//! 版本化嵌套：`/api/v1/*`。新增版本在此处扩展。

pub mod v1;

use axum::Router;

use crate::state::AppState;

/// 构建 API 路由
pub fn router() -> Router<AppState> {
    Router::new().nest("/api/v1", v1::router())
}
