//! 主播订阅业务编排：解析抖音号、管理订阅、检测开播

use crate::config::constants;
use crate::domain::streamer::Streamer;
use crate::domain::subscription::SubscriptionItem;
use crate::error::AppError;
use crate::state::AppState;
use crate::utils::id;
use crate::utils::time;

pub struct StreamerService;

impl StreamerService {
    /// 解析抖音号并订阅主播
    ///
    /// 通过 StreamerResolver 把抖音号转换为 sec_uid / nickname / avatar，
    /// 解析时 enter 接口会顺便返回开播状态，同步写入并返回给前端。
    /// 首次订阅关系建立时：
    /// - 主播 popularity +1
    /// - 在 streamer_poll_tasks 表中创建轮询任务（调度器后台自动检测开播）
    pub async fn subscribe(
        state: &AppState,
        user_id: &str,
        douyin_id: &str,
    ) -> Result<SubscriptionItem, AppError> {
        let resolved = state.streamer_resolver.resolve(douyin_id).await?;
        let now = time::now_ts();
        let live_started_at = if resolved.is_live { Some(now) } else { None };

        let streamer = match state
            .subscription_store
            .find_by_sec_uid(&resolved.sec_uid)
            .await?
        {
            Some(mut s) => {
                s.douyin_id = douyin_id.to_string();
                s.nickname = resolved.nickname;
                s.avatar = resolved.avatar;
                s.live = resolved.is_live;
                s.live_started_at = live_started_at;
                state.subscription_store.save_streamer(s.clone()).await?;
                s
            }
            None => {
                let s = Streamer {
                    id: id::gen_streamer_id(),
                    sec_uid: resolved.sec_uid.clone(),
                    douyin_id: douyin_id.to_string(),
                    nickname: resolved.nickname,
                    avatar: resolved.avatar,
                    live: resolved.is_live,
                    live_started_at,
                    popularity: 0,
                };
                state.subscription_store.save_streamer(s.clone()).await?;
                s
            }
        };

        let is_new = state
            .subscription_store
            .subscribe(user_id, &streamer.id, now)
            .await?;
        if is_new {
            let _ = state.subscription_store.inc_popularity(&streamer.id).await;
            // 新主播：创建轮询任务（幂等），让后台调度器自动检测开播状态
            let _ = state.poll_store.ensure_task(&streamer.id).await;
        }

        Ok(SubscriptionItem {
            streamer,
            subscribed_at: now,
        })
    }

    /// 列出当前用户订阅的主播
    pub async fn list(
        state: &AppState,
        user_id: &str,
    ) -> Result<Vec<SubscriptionItem>, AppError> {
        state.subscription_store.list_subscriptions(user_id).await
    }

    /// 列出热门主播
    pub async fn list_popular(
        state: &AppState,
        limit: i64,
    ) -> Result<Vec<Streamer>, AppError> {
        state.subscription_store.list_popular(limit).await
    }

    /// 取消订阅
    pub async fn unsubscribe(
        state: &AppState,
        user_id: &str,
        streamer_id: &str,
    ) -> Result<(), AppError> {
        state
            .subscription_store
            .unsubscribe(user_id, streamer_id)
            .await
    }

    /// 检测单个主播开播状态
    pub async fn check_live(
        state: &AppState,
        streamer_id: &str,
    ) -> Result<(Streamer, Option<String>), AppError> {
        let streamer = state
            .subscription_store
            .get_streamer(streamer_id)
            .await?
            .ok_or_else(|| AppError::not_found("主播不存在"))?;

        let status = state.live_checker.check(&streamer.douyin_id).await?;
        let became_live = !streamer.live && status.is_live;
        let now = time::now_ts();

        let updated = state
            .subscription_store
            .set_live(
                streamer_id,
                status.is_live,
                if status.is_live { Some(now) } else { None },
                status.nickname.as_deref(),
                status.avatar.as_deref(),
            )
            .await?
            .unwrap_or(streamer);

        let msg = if became_live {
            Some(live_notify_msg(&updated.nickname))
        } else {
            None
        };
        Ok((updated, msg))
    }

    /// 批量检测当前用户所有订阅主播
    pub async fn poll(
        state: &AppState,
        user_id: &str,
    ) -> Result<Vec<(Streamer, String)>, AppError> {
        let list = state.subscription_store.list_subscriptions(user_id).await?;
        let mut notifies = Vec::new();
        for item in list {
            // 串行避免高频请求触发风控
            let status = match state.live_checker.check(&item.streamer.douyin_id).await {
                Ok(st) => st,
                Err(e) => {
                    tracing::warn!("check live failed for {}: {:?}", item.streamer.douyin_id, e);
                    continue;
                }
            };
            let became_live = !item.streamer.live && status.is_live;
            let now = time::now_ts();
            if let Some(updated) = state
                .subscription_store
                .set_live(
                    &item.streamer.id,
                    status.is_live,
                    if status.is_live { Some(now) } else { None },
                    status.nickname.as_deref(),
                    status.avatar.as_deref(),
                )
                .await?
            {
                if became_live {
                    let nickname = updated.nickname.clone();
                    notifies.push((updated, live_notify_msg(&nickname)));
                }
            }
        }
        Ok(notifies)
    }
}

/// 开播通知文案
fn live_notify_msg(nickname: &str) -> String {
    constants::LIVE_NOTIFY_TEMPLATE.replace("{}", nickname)
}
