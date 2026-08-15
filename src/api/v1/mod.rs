//! v1 版本接口聚合
//!
//! 按受众拆分两组路由：
//! - `/api/v1/app/*`：移动客户端接口（短信登录、订阅、通知等）
//! - `/api/v1/admin/*`：管理员后台接口（用户名密码登录、主播收录运营等）

use axum::Router;

use crate::state::AppState;

pub mod admin;
pub mod app;

/// 构建 v1 路由
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/app", app::router(state.clone()))
        .nest("/admin", admin::router(state))
}
