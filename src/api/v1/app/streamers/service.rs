//! 主播订阅业务编排：订阅热门主播、想看意愿收集、检测开播

use crate::config::constants;
use crate::domain::streamer::Streamer;
use crate::domain::subscription::SubscriptionItem;
use crate::error::AppError;
use crate::state::AppState;
use crate::utils::douyin_id;
use crate::utils::time;

pub struct StreamerService;

impl StreamerService {
    /// 按抖音号订阅主播（已停用）
    ///
    /// 订阅机制已改为仅支持从热门主播列表中选择，手动输入抖音号改为
    /// 提交"想看"意愿（见 `add_wish`）。此接口保留但直接返回暂不支持。
    pub async fn subscribe(
        _state: &AppState,
        _user_id: &str,
        _douyin_id: &str,
    ) -> Result<SubscriptionItem, AppError> {
        Err(AppError::invalid_param(
            "暂不支持手动订阅主播，请从热门主播中选择",
        ))
    }

    /// 按主播 ID 订阅热门主播
    ///
    /// 主播必须已在 streamers 表中（即热门主播），否则返回 404。
    /// 首次订阅关系建立时 popularity +1 并确保轮询任务存在。
    pub async fn subscribe_by_id(
        state: &AppState,
        user_id: &str,
        streamer_id: &str,
    ) -> Result<SubscriptionItem, AppError> {
        let streamer = state
            .subscription_store
            .get_streamer(streamer_id)
            .await?
            .ok_or_else(|| AppError::not_found("主播不存在"))?;

        let now = time::now_ts();
        let is_new = state
            .subscription_store
            .subscribe(user_id, &streamer.id, now)
            .await?;
        if is_new {
            let _ = state.subscription_store.inc_popularity(&streamer.id).await;
            // 确保轮询任务存在（热门主播通常已有，此处幂等保证）
            let _ = state.poll_store.ensure_task(&streamer.id).await;
        }

        Ok(SubscriptionItem {
            streamer,
            subscribed_at: now,
        })
    }

    /// 提交想看意愿
    ///
    /// 用户输入想看的主播抖音号，按 douyin_id 去重并累加想看计数，
    /// 运营据此决定是否将该主播加入热门列表。
    pub async fn add_wish(state: &AppState, douyin_id: &str) -> Result<i64, AppError> {
        let trimmed = douyin_id.trim();
        douyin_id::validate(trimmed)?;
        let count = state.wish_store.upsert_wish(trimmed).await?;
        Ok(count)
    }

    /// 列出当前用户订阅的主播
    pub async fn list(state: &AppState, user_id: &str) -> Result<Vec<SubscriptionItem>, AppError> {
        state.subscription_store.list_subscriptions(user_id).await
    }

    /// 列出热门主播
    pub async fn list_popular(state: &AppState, limit: i64) -> Result<Vec<Streamer>, AppError> {
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
