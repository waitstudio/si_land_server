//! 轮询调度器
//!
//! 数据库驱动调度模式：单个常驻循环 + 短期异步任务。
//! - 每 [PollConfig::loop_interval_secs] 查询 next_poll_at <= now 且 poll_enabled=TRUE 的任务
//! - 用 FOR UPDATE SKIP LOCKED 避免多实例重复争抢
//! - 用 tokio Semaphore 限制抖音接口全局并发
//! - 自适应间隔：直播短间隔、未播常规、失败指数退避 + 随机抖动
//! - 状态持久化到 Postgres，重启可自动恢复

use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::config::AppConfig;
use crate::domain::scheduler::PollTask;
use crate::error::AppError;
use crate::services::douyin::live_checker::LiveChecker;
use crate::services::notice_store::NoticeStore;
use crate::services::push::{PushMessage, PushProvider, PushTokenStore};
use crate::services::scheduler::adaptive_interval::next_poll_at;
use crate::services::scheduler::poll_store::PollStore;
use crate::services::subscription_store::SubscriptionStore;
use crate::services::ws_hub::WsHub;
use crate::utils::time;

/// 轮询调度器
///
/// 持有所有依赖的 Arc，通过 [Self::run] 启动常驻循环。
/// main.rs 中通过 `tokio::spawn(scheduler.run())` 启动，
/// 程序退出时 task 被取消，状态已持久化到 DB，重启自动恢复。
pub struct PollScheduler {
    cfg: Arc<AppConfig>,
    poll_store: Arc<dyn PollStore>,
    subscription_store: Arc<dyn SubscriptionStore>,
    live_checker: Arc<dyn LiveChecker>,
    push_providers: Vec<Arc<dyn PushProvider>>,
    push_token_store: Arc<dyn PushTokenStore>,
    notice_store: Arc<dyn NoticeStore>,
    /// WebSocket 连接管理器：在线用户实时推送
    ws_hub: Arc<WsHub>,
    /// 全局并发信号量：限制同时调用抖音接口的请求数
    semaphore: Arc<Semaphore>,
}

impl PollScheduler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cfg: Arc<AppConfig>,
        poll_store: Arc<dyn PollStore>,
        subscription_store: Arc<dyn SubscriptionStore>,
        live_checker: Arc<dyn LiveChecker>,
        push_providers: Vec<Arc<dyn PushProvider>>,
        push_token_store: Arc<dyn PushTokenStore>,
        notice_store: Arc<dyn NoticeStore>,
        ws_hub: Arc<WsHub>,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(cfg.poll.max_concurrency));
        Self {
            cfg,
            poll_store,
            subscription_store,
            live_checker,
            push_providers,
            push_token_store,
            notice_store,
            ws_hub,
            semaphore,
        }
    }

    /// 启动常驻调度循环
    ///
    /// 每隔 loop_interval_secs 秒抓取一批到期任务，为每个任务生成短期异步任务执行检测。
    /// 循环本身不阻塞业务逻辑，所有检测并发执行（受 semaphore 限制）。
    pub async fn run(self: Arc<Self>) {
        let interval = std::time::Duration::from_secs(self.cfg.poll.loop_interval_secs);
        tracing::info!(
            "轮询调度器启动，间隔 {}s，并发上限 {}",
            self.cfg.poll.loop_interval_secs,
            self.cfg.poll.max_concurrency
        );

        loop {
            if let Err(e) = Self::tick(Arc::clone(&self)).await {
                tracing::warn!("调度循环异常: {:?}", e);
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// 单次调度：抓取一批任务并并发执行检测
    async fn tick(self: Arc<Self>) -> Result<(), AppError> {
        let now = time::now_ts();
        let tasks = self
            .poll_store
            .fetch_due(now, self.cfg.poll.batch_size)
            .await?;

        if tasks.is_empty() {
            return Ok(());
        }

        tracing::debug!("本轮调度 {} 个主播", tasks.len());

        let mut handles = Vec::with_capacity(tasks.len());
        for task in tasks {
            let scheduler = Arc::clone(&self);
            let permit = scheduler.semaphore.clone();
            handles.push(tokio::spawn(async move {
                // acquire_permit 在闭包内执行，避免 semaphore 持有过久
                let _permit = match permit.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("Semaphore 已关闭");
                        return;
                    }
                };
                if let Err(e) = scheduler.process_task(&task).await {
                    tracing::warn!(
                        "处理主播 {} 失败: {:?}",
                        task.streamer_id, e
                    );
                }
            }));
        }

        // 等待本轮所有任务完成（不阻塞下一轮调度，下一轮仍受 semaphore 限制）
        for h in handles {
            let _ = h.await;
        }
        Ok(())
    }

    /// 处理单个主播的轮询任务
    async fn process_task(&self, task: &PollTask) -> Result<(), AppError> {
        let streamer = self
            .subscription_store
            .get_streamer(&task.streamer_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("主播 {} 不存在", task.streamer_id)))?;

        // 设置超时保护，避免单次请求卡住整个调度
        let check_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.cfg.poll.check_timeout_secs),
            self.live_checker.check(&streamer.douyin_id),
        )
        .await;

        // 成功检测时携带抖音返回的昵称/头像/直播间标题用于同步更新；
        // 失败或超时时不更新资料（None 保持原值），仅累加 fail_count。
        let (is_live, fail_count, nickname, avatar, room_title) = match check_result {
            Ok(Ok(status)) => (
                status.is_live,
                0,
                status.nickname,
                status.avatar,
                status.room_title,
            ),
            Ok(Err(e)) => {
                tracing::warn!(
                    "检测 {} 开播状态失败: {:?}",
                    streamer.douyin_id, e
                );
                (task.is_live(), task.fail_count + 1, None, None, None)
            }
            Err(_) => {
                tracing::warn!("检测 {} 超时", streamer.douyin_id);
                (task.is_live(), task.fail_count + 1, None, None, None)
            }
        };

        // 持久化状态到 streamers 表（同时同步昵称/头像）
        let became_live = !task.is_live() && is_live;
        let now = time::now_ts();
        let live_started_at = if is_live { Some(now) } else { None };
        let _ = self
            .subscription_store
            .set_live(
                &streamer.id,
                is_live,
                live_started_at,
                nickname.as_deref(),
                avatar.as_deref(),
            )
            .await?;

        // 计算下次轮询时间并持久化调度任务
        let next = next_poll_at(now, is_live, fail_count, &self.cfg.poll);
        let last_status: i16 = if fail_count > 0 {
            task.last_status // 失败时不更新 last_status
        } else if is_live {
            1
        } else {
            2
        };
        self.poll_store
            .schedule_next(&task.streamer_id, next, last_status, fail_count)
            .await?;

        // 状态从【未播→开播】时，先入库通知再按在线/离线分流推送
        if became_live {
            self.notify_subscribers(
                &streamer.nickname,
                Some(streamer.avatar.as_str()),
                &task.streamer_id,
                live_started_at,
                room_title.as_deref(),
            )
            .await;
        }
        Ok(())
    }

    /// 主播开播时，查询所有订阅者，先入库一条开播通知，再分流推送：
    /// - 在线用户（WS 有存活连接）：推送 notice + 权威未读数，实时弹窗
    /// - 离线用户：按绑定的推送通道（如 Bark）走系统推送
    ///
    /// 入库与推送均失败不影响主流程，仅记录日志；
    /// 即使用户未绑定推送凭证，通知仍会入库，可在 App 内消息列表看到。
    async fn notify_subscribers(
        &self,
        nickname: &str,
        avatar: Option<&str>,
        streamer_id: &str,
        live_started_at: Option<i64>,
        room_title: Option<&str>,
    ) {
        let user_ids = match self.poll_store.list_subscribers(streamer_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("查询订阅者失败: {:?}", e);
                return;
            }
        };
        if user_ids.is_empty() {
            return;
        }

        // 通知文案：能取到直播间标题时带上标题，取不到时回退到通用文案
        let title = "主播开播".to_string();
        let body = match room_title {
            Some(t) if !t.trim().is_empty() => {
                format!("正在直播：{}", t.trim())
            }
            _ => format!("{} 正在直播，快来看吧", nickname),
        };
        let created_at = time::now_ts();

        // 分流判定：以入库时刻的 WS 连接状态为准
        let (online_users, offline_users): (Vec<String>, Vec<String>) = user_ids
            .into_iter()
            .partition(|uid| self.ws_hub.is_online(uid));

        // 先入库：为每个订阅者落库一条通知（即使后续推送失败也能在 App 内看到）。
        // 入库返回的通知实体直接用于在线用户的 WS 推送。
        let mut online_notices: Vec<(String, crate::domain::notice::LiveNotice)> =
            Vec::with_capacity(online_users.len());
        for uid in &online_users {
            match self
                .notice_store
                .create(
                    uid,
                    streamer_id,
                    nickname,
                    &title,
                    &body,
                    live_started_at,
                    created_at,
                )
                .await
            {
                Ok(notice) => online_notices.push((uid.clone(), notice)),
                Err(e) => tracing::warn!("入库开播通知失败 user={}: {:?}", uid, e),
            }
        }
        for uid in &offline_users {
            if let Err(e) = self
                .notice_store
                .create(
                    uid,
                    streamer_id,
                    nickname,
                    &title,
                    &body,
                    live_started_at,
                    created_at,
                )
                .await
            {
                tracing::warn!("入库开播通知失败 user={}: {:?}", uid, e);
            }
        }

        // 在线用户：WS 推送 notice（字段与 NoticeItem 对齐，snake_case）
        // + 权威未读数（客户端收到后校准红点，容错本地计数漂移）
        for (uid, notice) in online_notices {
            let payload = serde_json::json!({
                "type": "notice",
                "data": {
                    "id": notice.id,
                    "streamer_id": notice.streamer_id,
                    "streamer_nickname": notice.streamer_nickname,
                    "avatar": avatar,
                    "title": notice.title,
                    "body": notice.body,
                    "live_started_at": notice.live_started_at,
                    "created_at": notice.created_at,
                    "read": false,
                }
            });
            self.ws_hub.send_to_user(&uid, &payload.to_string());
            if let Ok(unread) = self.notice_store.unread_count(&uid).await {
                let sync = serde_json::json!({"type": "unread", "data": {"count": unread}});
                self.ws_hub.send_to_user(&uid, &sync.to_string());
            }
        }

        // 离线用户：按绑定的推送通道下发（Bark 系统推送）
        if offline_users.is_empty() {
            return;
        }
        let tokens = match self.push_token_store.list_by_users(&offline_users).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("查询推送凭证失败: {:?}", e);
                return;
            }
        };

        let msg = PushMessage {
            title,
            body,
            url: None,
        };

        for token in tokens {
            let provider = self
                .push_providers
                .iter()
                .find(|p| p.channel() == token.channel);
            match provider {
                Some(p) => {
                    if let Err(e) = p.send(&token.token, &msg).await {
                        tracing::warn!(
                            "推送失败 channel={} user={}: {:?}",
                            token.channel,
                            token.user_id,
                            e
                        );
                    }
                }
                None => {
                    tracing::warn!(
                        "未配置 {} 推送通道，跳过用户 {}",
                        token.channel,
                        token.user_id
                    );
                }
            }
        }
    }
}
