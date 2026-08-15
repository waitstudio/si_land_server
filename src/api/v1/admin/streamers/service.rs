//! 管理员主播收录业务编排
//!
//! 原 App 端"按抖音号手动订阅"的解析与入库逻辑迁移至此：
//! 抖音号 → 解析 sec_uid/昵称/头像 → 查重 → 入库 → 建立轮询任务。

use crate::config::constants;
use crate::domain::streamer::Streamer;
use crate::error::AppError;
use crate::response::BizCode;
use crate::state::AppState;
use crate::utils::{douyin_id, id, time};

pub struct AdminStreamerService;

impl AdminStreamerService {
    /// 按抖音号收录主播
    ///
    /// 1. 校验抖音号格式
    /// 2. 调用抖音 enter 接口解析 sec_uid / 昵称 / 头像 / 开播状态
    /// 3. 按 sec_uid 查重，已收录则返回 Conflict
    /// 4. 写入 streamers 表并确保轮询任务存在（收录即进入开播检测）
    pub async fn add_streamer(
        state: &AppState,
        douyin_id: &str,
    ) -> Result<Streamer, AppError> {
        let trimmed = douyin_id.trim();
        douyin_id::validate(trimmed)?;

        let resolved = state.streamer_resolver.resolve(trimmed).await?;

        if let Some(existing) = state
            .subscription_store
            .find_by_sec_uid(&resolved.sec_uid)
            .await?
        {
            return Err(AppError::new(
                BizCode::Conflict,
                format!("该主播已收录：{}", existing.nickname),
            ));
        }

        let now = time::now_ts();
        let streamer = Streamer {
            id: id::gen_streamer_id(),
            sec_uid: resolved.sec_uid,
            douyin_id: trimmed.to_string(),
            nickname: resolved.nickname,
            avatar: resolved.avatar,
            live: resolved.is_live,
            live_started_at: if resolved.is_live { Some(now) } else { None },
            popularity: 0,
        };
        state.subscription_store.save_streamer(streamer.clone()).await?;
        // 收录即开始轮询开播状态（幂等）
        let _ = state.poll_store.ensure_task(&streamer.id).await;

        Ok(streamer)
    }

    /// 已收录主播列表（按人气降序，最多前 100）
    pub async fn list_streamers(state: &AppState) -> Result<Vec<Streamer>, AppError> {
        state
            .subscription_store
            .list_popular(constants::POPULAR_MAX_LIMIT)
            .await
    }
}
