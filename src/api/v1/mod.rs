//! v1 版本接口聚合
//!
//! 新增功能模块时：在 `v1/` 下新建目录（含 handler/dto/service），
//! 在此处注册路由即可，互不影响。

use axum::{routing::post, Router};

use crate::state::AppState;

pub mod auth;
pub mod sms;

/// 构建 v1 路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sms/send", post(sms::send_sms))
        .route("/auth/login", post(auth::login))
}
