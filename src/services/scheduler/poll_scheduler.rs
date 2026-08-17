//! 轮询调度器（Redis 驱动）
//!
//! 取出-执行-放回模式：
//! - 启动对账：streamers 全表同步任务队列（bootstrap）
//! - 每 [PollConfig::loop_interval_secs] 领取到期任务（claim，ZREM 原子抢占）
//! - 检测后按自适应间隔归还（complete），实例崩溃由 inflight 兜底回收
//! - 主播快照缓存于 Redis：与抖音接口结果对比，仅在变更时写 DB 并失效缓存
//! - 用 tokio Semaphore 限制抖音接口全局并发

use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::services::douyin::live_checker::LiveChecker;
use crate::services::notice_store::NoticeStore;
use crate::services::notification_queue::NotificationQueue;
use crate::services::scheduler::adaptive_interval::next_poll_at;
use crate::services::scheduler::redis_poll_store::{PollSnapshot, PollStore};
use crate::services::subscription_store::SubscriptionStore;
use crate::utils::time;

/// 意外失败后的兜底归还间隔（固定 60s，正常退避由快照 fail_count 驱动）
const FAILURE_FALLBACK_SECS: i64 = 60;

/// 轮询调度器
///
/// main.rs 中通过 `tokio::spawn(scheduler.run())` 启动，
/// 程序退出时 task 被取消，任务状态在 Redis，重启自动恢复。
pub struct PollScheduler {
    cfg: Arc<AppConfig>,
    poll_store: Arc<dyn PollStore>,
    subscription_store: Arc<dyn SubscriptionStore>,
    live_checker: Arc<dyn LiveChecker>,
    notice_store: Arc<dyn NoticeStore>,
    notification_queue: Arc<dyn NotificationQueue>,
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
        notice_store: Arc<dyn NoticeStore>,
        notification_queue: Arc<dyn NotificationQueue>,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(cfg.poll.max_concurrency));
        Self {
            cfg,
            poll_store,
            subscription_store,
            live_checker,
            notice_store,
            notification_queue,
            semaphore,
        }
    }

    /// 启动常驻调度循环
    ///
    /// 先执行启动对账（streamers 全表 → Redis 任务队列），
    /// 再进入领取-处理-归还循环，每轮顺带回收超时 inflight 任务。
    pub async fn run(self: Arc<Self>) {
        tracing::info!(
            "轮询调度器启动（Redis 驱动），间隔 {}s，并发上限 {}",
            self.cfg.poll.loop_interval_secs,
            self.cfg.poll.max_concurrency
        );

        match self.poll_store.bootstrap().await {
            Ok(count) => tracing::info!("启动对账完成，{} 个主播纳入轮询", count),
            Err(error) => tracing::error!(?error, "启动对账失败，等待下一轮对账"),
        }

        let interval = std::time::Duration::from_secs(self.cfg.poll.loop_interval_secs);
        loop {
            if let Err(e) = Self::tick(Arc::clone(&self)).await {
                tracing::warn!("调度循环异常: {:?}", e);
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// 单次调度：回收超时任务 → 领取一批 → 并发执行检测
    async fn tick(self: Arc<Self>) -> Result<(), AppError> {
        let now = time::now_ts();

        // 实例崩溃兜底：超时未归还的任务放回队列
        let recovered = self.poll_store.recover_inflight(now).await?;
        if recovered > 0 {
            tracing::warn!("回收 {} 个超时未归还的轮询任务", recovered);
        }

        let streamer_ids = self
            .poll_store
            .claim(
                now,
                self.cfg.poll.batch_size,
                self.cfg.poll.check_timeout_secs.saturating_add(30) as i64,
            )
            .await?;

        if streamer_ids.is_empty() {
            return Ok(());
        }

        tracing::debug!("本轮调度 {} 个主播", streamer_ids.len());

        let mut handles = Vec::with_capacity(streamer_ids.len());
        for streamer_id in streamer_ids {
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
                if let Err(e) = scheduler.process_task(&streamer_id).await {
                    tracing::warn!("处理主播 {} 失败: {:?}", streamer_id, e);
                    // 兜底归还：保证任务不滞留 inflight 等待超时回收
                    let fallback = time::now_ts() + FAILURE_FALLBACK_SECS;
                    if let Err(recover_error) =
                        scheduler.poll_store.complete(&streamer_id, fallback).await
                    {
                        tracing::error!(
                            "兜底归还主播 {} 任务失败: {:?}",
                            streamer_id,
                            recover_error
                        );
                    }
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
    ///
    /// 快照 miss 时从 streamers 表回填；与抖音接口结果对比，
    /// 仅在变更时写 DB 并失效缓存，无变更只更新快照内的轮询状态。
    async fn process_task(&self, streamer_id: &str) -> Result<(), AppError> {
        let mut snapshot = match self.poll_store.get_snapshot(streamer_id).await? {
            Some(s) => s,
            None => {
                let streamer = self
                    .subscription_store
                    .get_streamer(streamer_id)
                    .await?
                    .ok_or_else(|| AppError::not_found(format!("主播 {} 不存在", streamer_id)))?;
                let snapshot = PollSnapshot::from(streamer);
                self.poll_store.save_snapshot(&snapshot).await?;
                snapshot
            }
        };

        // 设置超时保护，避免单次请求卡住整个调度
        let check_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.cfg.poll.check_timeout_secs),
            self.live_checker.check(&snapshot.douyin_id),
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
                tracing::warn!("检测 {} 开播状态失败: {:?}", snapshot.douyin_id, e);
                (snapshot.live, snapshot.fail_count + 1, None, None, None)
            }
            Err(_) => {
                tracing::warn!("检测 {} 超时", snapshot.douyin_id);
                (snapshot.live, snapshot.fail_count + 1, None, None, None)
            }
        };

        let became_live = !snapshot.live && is_live;
        let now = time::now_ts();

        // 变更判定：开播状态切换，或昵称/头像与快照不一致（抖音未返回的空值跳过）
        let profile_changed = nickname
            .as_deref()
            .is_some_and(|n| !n.is_empty() && n != snapshot.nickname)
            || avatar
                .as_deref()
                .is_some_and(|a| !a.is_empty() && a != snapshot.avatar);
        if is_live != snapshot.live || profile_changed {
            // 开播时间：未播→在播取 now；在播持续保留原值（首次开播时间不被覆盖）
            let live_started_at = if is_live {
                Some(snapshot.live_started_at.unwrap_or(now))
            } else {
                None
            };
            self.subscription_store
                .set_live(
                    streamer_id,
                    is_live,
                    live_started_at,
                    nickname.as_deref(),
                    avatar.as_deref(),
                )
                .await?;
            // DB 已更新，删除缓存快照，下轮回填最新值
            self.poll_store.invalidate_snapshot(streamer_id).await?;
        } else {
            // 无变更：不写 DB，仅更新快照轮询状态
            snapshot.live = is_live;
            snapshot.fail_count = fail_count;
            self.poll_store.save_snapshot(&snapshot).await?;
        }

        // 按自适应间隔归还任务
        let next = next_poll_at(now, is_live, fail_count, &self.cfg.poll);
        self.poll_store.complete(streamer_id, next).await?;

        // 状态从【未播→开播】时，先入库通知再按在线/离线分流推送
        if became_live {
            let nickname_for_notice = nickname.unwrap_or_else(|| snapshot.nickname.clone());
            let avatar_for_notice = avatar.unwrap_or_else(|| snapshot.avatar.clone());
            self.notify_subscribers(
                &nickname_for_notice,
                Some(avatar_for_notice.as_str()),
                streamer_id,
                Some(now),
                room_title.as_deref(),
            )
            .await;
        }
        Ok(())
    }

    /// 主播开播时先落库通知并发布到 Kafka；实际 WS/系统推送由 Worker 异步完成。
    async fn notify_subscribers(
        &self,
        nickname: &str,
        avatar: Option<&str>,
        streamer_id: &str,
        live_started_at: Option<i64>,
        room_title: Option<&str>,
    ) {
        let user_ids = match self.subscription_store.list_subscribers(streamer_id).await {
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

        for user_id in user_ids {
            let notice = match self
                .notice_store
                .create(
                    &user_id,
                    streamer_id,
                    nickname,
                    &title,
                    &body,
                    live_started_at,
                    created_at,
                )
                .await
            {
                Ok(notice) => notice,
                Err(error) => {
                    tracing::warn!(user_id, ?error, "创建开播通知失败");
                    continue;
                }
            };
            let payload = serde_json::json!({
                "id": notice.id,
                "streamer_id": notice.streamer_id,
                "streamer_nickname": notice.streamer_nickname,
                "avatar": avatar,
                "title": notice.title,
                "body": notice.body,
                "live_started_at": notice.live_started_at,
                "created_at": notice.created_at,
                "read": false,
            });
            if let Err(error) = self.notification_queue.publish(&user_id, payload).await {
                tracing::error!(user_id, ?error, "发布通知消息失败");
            }
        }
    }
}
