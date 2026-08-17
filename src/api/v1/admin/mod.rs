//! Admin 端接口（面向管理后台）
//!
//! - 公开路由：管理员登录 / 登出
//! - 管理员路由：经 `auth_middleware` 校验 JWT 后，再由 `admin_guard`
//!   校验 token 主体为内置管理员，App 端用户 token 无法访问

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, post};

use crate::middleware::auth::{admin_guard, auth_middleware};
use crate::state::AppState;

pub mod auth;
pub mod streamers;
pub mod wishes;

/// 构建 Admin 端路由（挂载于 /api/v1/admin）
pub fn router(state: AppState) -> Router<AppState> {
    // 公开路由
    let public = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout));

    // 需管理员身份的路由
    let protected = Router::new()
        .route("/user/info", get(auth::user_info))
        .route("/auth/codes", get(auth::codes))
        .route("/menu/all", get(auth::menus))
        // 主播收录运营
        .route(
            "/streamers",
            post(streamers::add_streamer).get(streamers::list_streamers),
        )
        // 想看意愿查询
        .route("/wishes", get(wishes::list_wishes));

    Router::new().merge(public).merge(
        protected
            // route_layer 为洋葱模型：后添加的在外层。
            // 请求先过 auth_middleware（校验 JWT 并注入 UserId），再过 admin_guard。
            .route_layer(from_fn(admin_guard))
            .route_layer(from_fn_with_state(state, auth_middleware)),
    )
}
