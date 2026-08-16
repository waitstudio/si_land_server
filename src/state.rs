//! 应用状态与依赖注入

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::code_store::{CodeStore, InMemoryCodeStore};
use crate::services::db;
use crate::services::douyin::client::DouyinEnterClient;
use crate::services::douyin::live_checker::{HttpLiveChecker, LiveChecker};
use crate::services::douyin::streamer_resolver::{HttpStreamerResolver, StreamerResolver};
use crate::services::feedback_store::{FeedbackStore, PgFeedbackStore};
use crate::services::notice_store::{NoticeStore, PgNoticeStore};
use crate::services::push::bark::BarkProvider;
use crate::services::push::provider::PushProvider;
use crate::services::push::token_store::{PgPushTokenStore, PushTokenStore};
use crate::services::scheduler::poll_store::{PgPollStore, PollStore};
use crate::services::scheduler::PollScheduler;
use crate::services::sms_provider::{MockSmsProvider, SmsProvider};
use crate::services::streamer_wish_store::{PgStreamerWishStore, StreamerWishStore};
use crate::services::subscription_store::{PgSubscriptionStore, SubscriptionStore};
use crate::services::user_store::{PgUserStore, UserStore};
use crate::services::ws_hub::WsHub;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub sms_provider: Arc<dyn SmsProvider>,
    pub code_store: Arc<dyn CodeStore>,
    pub user_store: Arc<dyn UserStore>,
    pub subscription_store: Arc<dyn SubscriptionStore>,
    pub streamer_resolver: Arc<dyn StreamerResolver>,
    pub live_checker: Arc<dyn LiveChecker>,
    pub poll_store: Arc<dyn PollStore>,
    pub push_token_store: Arc<dyn PushTokenStore>,
    pub push_providers: Vec<Arc<dyn PushProvider>>,
    pub notice_store: Arc<dyn NoticeStore>,
    pub wish_store: Arc<dyn StreamerWishStore>,
    /// 问题反馈存储
    pub feedback_store: Arc<dyn FeedbackStore>,
    /// WebSocket 连接管理器（实时通知）
    pub ws_hub: Arc<WsHub>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: AppConfig,
        sms_provider: Arc<dyn SmsProvider>,
        code_store: Arc<dyn CodeStore>,
        user_store: Arc<dyn UserStore>,
        subscription_store: Arc<dyn SubscriptionStore>,
        streamer_resolver: Arc<dyn StreamerResolver>,
        live_checker: Arc<dyn LiveChecker>,
        poll_store: Arc<dyn PollStore>,
        push_token_store: Arc<dyn PushTokenStore>,
        push_providers: Vec<Arc<dyn PushProvider>>,
        notice_store: Arc<dyn NoticeStore>,
        wish_store: Arc<dyn StreamerWishStore>,
        feedback_store: Arc<dyn FeedbackStore>,
        ws_hub: Arc<WsHub>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            sms_provider,
            code_store,
            user_store,
            subscription_store,
            streamer_resolver,
            live_checker,
            poll_store,
            push_token_store,
            push_providers,
            notice_store,
            wish_store,
            feedback_store,
            ws_hub,
        }
    }
}

/// 组装默认 AppState（PostgreSQL 持久化实现）
///
/// 启动时：
/// - 建立 PgPool 并执行建表迁移
/// - 用 Pg 实现 UserStore / SubscriptionStore / PollStore / PushTokenStore
/// - 共享 `DouyinEnterClient` 给 resolver / live_checker
/// - 注册 Bark 推送通道（APNs 等其他通道在此追加）
pub async fn build_state(config: AppConfig) -> Result<AppState, AppError> {
    let pool = db::init_pool(&config.database.url, config.server.db_max_connections).await?;
    db::migrations::run(&pool).await?;

    let user_store: Arc<dyn UserStore> = Arc::new(PgUserStore::new(pool.clone()));
    let subscription_store: Arc<dyn SubscriptionStore> = Arc::new(PgSubscriptionStore::new(pool.clone()));
    let poll_store: Arc<dyn PollStore> = Arc::new(PgPollStore::new(pool.clone()));
    let push_token_store: Arc<dyn PushTokenStore> = Arc::new(PgPushTokenStore::new(pool.clone()));
    let notice_store: Arc<dyn NoticeStore> = Arc::new(PgNoticeStore::new(pool.clone()));
    let wish_store: Arc<dyn StreamerWishStore> = Arc::new(PgStreamerWishStore::new(pool.clone()));
    let feedback_store: Arc<dyn FeedbackStore> = Arc::new(PgFeedbackStore::new(pool));
    let code_store: Arc<dyn CodeStore> = Arc::new(InMemoryCodeStore::new());
    let sms_provider: Arc<dyn SmsProvider> = Arc::new(MockSmsProvider);

    let douyin_client = Arc::new(DouyinEnterClient::new(Arc::new(config.douyin.clone()))?);
    let streamer_resolver: Arc<dyn StreamerResolver> =
        Arc::new(HttpStreamerResolver::new(douyin_client.clone()));
    let live_checker: Arc<dyn LiveChecker> = Arc::new(HttpLiveChecker::new(douyin_client));

    // 注册推送通道：Bark 默认启用，APNs 等在此追加
    let mut push_providers: Vec<Arc<dyn PushProvider>> = Vec::new();
    match BarkProvider::new(config.push.bark_base_url.clone(), config.push.timeout_secs) {
        Ok(b) => push_providers.push(Arc::new(b)),
        Err(e) => tracing::warn!("Bark 推送通道初始化失败: {:?}", e),
    }

    let ws_hub = Arc::new(WsHub::new());

    Ok(AppState::new(
        config,
        sms_provider,
        code_store,
        user_store,
        subscription_store,
        streamer_resolver,
        live_checker,
        poll_store,
        push_token_store,
        push_providers,
        notice_store,
        wish_store,
        feedback_store,
        ws_hub,
    ))
}

/// 启动轮询调度器后台任务
///
/// 在 main.rs 中调用，把调度器作为独立 tokio task 运行。
/// 程序退出时 task 被取消，状态已持久化到 DB，重启自动恢复。
pub fn spawn_scheduler(state: &AppState) {
    let scheduler = Arc::new(PollScheduler::new(
        Arc::clone(&state.config),
        Arc::clone(&state.poll_store),
        Arc::clone(&state.subscription_store),
        Arc::clone(&state.live_checker),
        state.push_providers.clone(),
        Arc::clone(&state.push_token_store),
        Arc::clone(&state.notice_store),
        Arc::clone(&state.ws_hub),
    ));
    tokio::spawn(scheduler.run());
}
