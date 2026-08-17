//! App 端接口（面向移动客户端）
//!
//! - 公开路由（login / sms/send）：无需认证
//! - 认证路由（auth/me / auth/nickname / streamers/* / push/*）：经过 `auth_middleware` 校验 JWT

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{delete, get, post, put};

use crate::middleware::auth::auth_middleware;
use crate::state::AppState;

pub mod auth;
pub mod feedback;
pub mod notices;
pub mod push;
pub mod sms;
pub mod streamers;
pub mod ws;

/// 构建 App 端路由（挂载于 /api/v1/app）
///
/// 接收 `state` 是为了通过 `from_fn_with_state` 把 state 注入到认证中间件，
/// 中间件再用 `State<AppState>` 提取。
pub fn router(state: AppState) -> Router<AppState> {
    // 公开路由
    let public = Router::new()
        .route("/sms/send", post(sms::send_sms))
        .route("/auth/login", post(auth::login))
        // WS 升级端点：token 走 query（握手无法携带 Header），升级前校验
        .route("/ws", get(ws::ws_handler));

    // 需认证路由
    let protected = Router::new()
        .route("/auth/me", get(auth::me))
        .route("/auth/nickname", put(auth::update_nickname))
        .route("/auth/ws-ticket", get(auth::issue_ws_ticket))
        // 主播订阅
        .route(
            "/streamers",
            post(streamers::add_subscription).get(streamers::list_subscriptions),
        )
        // 静态路径必须放在 {id} 之前，避免被动态路由捕获
        .route("/streamers/popular", get(streamers::list_popular))
        .route("/streamers/wishes", post(streamers::add_wish))
        .route("/streamers/poll", post(streamers::poll_live))
        .route("/streamers/{id}", delete(streamers::remove_subscription))
        .route("/streamers/{id}/check-live", post(streamers::check_live))
        .route(
            "/streamers/{id}/subscribe",
            post(streamers::subscribe_by_id),
        )
        // 推送凭证管理
        .merge(push::router())
        // 开播通知
        .merge(notices::router())
        // 问题反馈
        .route("/feedback", post(feedback::submit));

    Router::new()
        .merge(public)
        .merge(protected.route_layer(from_fn_with_state(state, auth_middleware)))
}
