//! 轮询任务存储：Redis ZSet 驱动（取出-执行-放回模式）
//!
//! 数据结构：
//! - `poll:tasks`     ZSet，member=streamer_id，score=下次检测时间（待执行队列）
//! - `poll:inflight`  ZSet，member=streamer_id，score=执行兜底到期时间（崩溃恢复）
//! - `poll:snap:{id}` String(JSON)，主播快照+轮询状态，TTL 7 天防僵尸
//!
//! streamers 表为唯一数据源：启动对账全量同步，运行期调度状态仅存 Redis。

use async_trait::async_trait;
use redis::{AsyncCommands, Script};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::domain::streamer::Streamer;
use crate::error::AppError;
use crate::utils::time;

const TASKS_KEY: &str = "poll:tasks";
const INFLIGHT_KEY: &str = "poll:inflight";
const SNAP_KEY_PREFIX: &str = "poll:snap:";
/// 快照 TTL：7 天兜底清理（正常路径持续续期）
const SNAP_TTL_SECS: u64 = 7 * 24 * 3600;

/// 领取任务：到期者从 tasks 取出（ZREM 抢占）并放入 inflight 兜底队列
const CLAIM_LUA: &str = r#"
local ids = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[2])
local claimed = {}
for _, id in ipairs(ids) do
  if redis.call('ZREM', KEYS[1], id) == 1 then
    redis.call('ZADD', KEYS[2], ARGV[3], id)
    table.insert(claimed, id)
  end
end
return claimed
"#;

/// 回收兜底：超时未归还的 inflight 任务放回 tasks（score=now，立即可领）
const RECOVER_LUA: &str = r#"
local ids = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1])
for _, id in ipairs(ids) do
  redis.call('ZREM', KEYS[1], id)
  redis.call('ZADD', KEYS[2], ARGV[1], id)
end
return #ids
"#;

/// 主播轮询快照（Redis 缓存 + 对比基准）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollSnapshot {
    pub streamer_id: String,
    pub douyin_id: String,
    pub live: bool,
    pub live_started_at: Option<i64>,
    pub nickname: String,
    pub avatar: String,
    /// 连续检测失败次数（成功清零），用于指数退避
    pub fail_count: i32,
}

impl From<Streamer> for PollSnapshot {
    fn from(streamer: Streamer) -> Self {
        Self {
            streamer_id: streamer.id,
            douyin_id: streamer.douyin_id,
            live: streamer.live,
            live_started_at: streamer.live_started_at,
            nickname: streamer.nickname,
            avatar: streamer.avatar,
            fail_count: 0,
        }
    }
}

/// 轮询任务存储抽象（Redis 驱动）
#[async_trait]
pub trait PollStore: Send + Sync {
    /// 启动对账：streamers 全表同步到任务队列（NX 只补缺失，不扰动现存调度）
    async fn bootstrap(&self) -> Result<usize, AppError>;
    /// 领取一批到期任务（原子），返回抢占成功的 streamer_id
    async fn claim(
        &self,
        now: i64,
        limit: i64,
        exec_timeout_secs: i64,
    ) -> Result<Vec<String>, AppError>;
    /// 归还任务：按下次检测时间放回队列并移出 inflight
    async fn complete(&self, streamer_id: &str, next_poll_at: i64) -> Result<(), AppError>;
    /// 回收超时未归还的任务（实例崩溃兜底），返回回收数
    async fn recover_inflight(&self, now: i64) -> Result<usize, AppError>;
    /// 主播订阅时加入轮询（幂等）
    async fn ensure_task(&self, streamer_id: &str) -> Result<(), AppError>;
    /// 读取主播快照（miss 返回 None，调用方从 DB 回填）
    async fn get_snapshot(&self, streamer_id: &str) -> Result<Option<PollSnapshot>, AppError>;
    /// 写入快照（TTL 续期）
    async fn save_snapshot(&self, snapshot: &PollSnapshot) -> Result<(), AppError>;
    /// 删除快照（DB 更新后失效缓存）
    async fn invalidate_snapshot(&self, streamer_id: &str) -> Result<(), AppError>;
}

pub struct RedisPollStore {
    client: redis::Client,
    pool: PgPool,
}

impl RedisPollStore {
    pub fn new(redis_url: &str, pool: PgPool) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)
            .map_err(|error| AppError::internal(format!("Redis 配置错误: {error}")))?;
        Ok(Self { client, pool })
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, AppError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::internal(format!("Redis 连接失败: {error}")))
    }

    fn snap_key(streamer_id: &str) -> String {
        format!("{SNAP_KEY_PREFIX}{streamer_id}")
    }
}

#[async_trait]
impl PollStore for RedisPollStore {
    async fn bootstrap(&self) -> Result<usize, AppError> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM streamers")
            .fetch_all(&self.pool)
            .await?;
        let ids: Vec<String> = rows.into_iter().map(|(id,)| id).collect();

        let mut conn = self.connection().await?;
        let now = time::now_ts();
        let mut pipe = redis::pipe();
        for id in &ids {
            // NX：仅补缺失，不覆盖现存 score（避免扰动退避/其他实例调度）
            pipe.cmd("ZADD")
                .arg(TASKS_KEY)
                .arg("NX")
                .arg(now)
                .arg(id)
                .ignore();
        }
        pipe.query_async::<()>(&mut conn)
            .await
            .map_err(|error| AppError::internal(format!("同步轮询任务队列失败: {error}")))?;
        Ok(ids.len())
    }

    async fn claim(
        &self,
        now: i64,
        limit: i64,
        exec_timeout_secs: i64,
    ) -> Result<Vec<String>, AppError> {
        let mut conn = self.connection().await?;
        Script::new(CLAIM_LUA)
            .key(TASKS_KEY)
            .key(INFLIGHT_KEY)
            .arg(now)
            .arg(limit)
            .arg(now + exec_timeout_secs)
            .invoke_async::<Vec<String>>(&mut conn)
            .await
            .map_err(|error| AppError::internal(format!("领取轮询任务失败: {error}")))
    }

    async fn complete(&self, streamer_id: &str, next_poll_at: i64) -> Result<(), AppError> {
        let mut conn = self.connection().await?;
        let mut pipe = redis::pipe();
        // 注意：redis-rs 的 zadd 参数顺序是 (key, member, score)，与 Redis 命令行相反
        pipe.zadd(TASKS_KEY, streamer_id, next_poll_at)
            .ignore()
            .zrem(INFLIGHT_KEY, streamer_id)
            .ignore();
        pipe.query_async::<()>(&mut conn)
            .await
            .map_err(|error| AppError::internal(format!("归还轮询任务失败: {error}")))?;
        Ok(())
    }

    async fn recover_inflight(&self, now: i64) -> Result<usize, AppError> {
        let mut conn = self.connection().await?;
        Script::new(RECOVER_LUA)
            .key(INFLIGHT_KEY)
            .key(TASKS_KEY)
            .arg(now)
            .invoke_async::<i64>(&mut conn)
            .await
            .map(|n| n as usize)
            .map_err(|error| AppError::internal(format!("回收 inflight 任务失败: {error}")))
    }

    async fn ensure_task(&self, streamer_id: &str) -> Result<(), AppError> {
        let mut conn = self.connection().await?;
        let now = time::now_ts();
        // NX：已存在则不覆盖（避免把退避中的任务提前）
        redis::cmd("ZADD")
            .arg(TASKS_KEY)
            .arg("NX")
            .arg(now)
            .arg(streamer_id)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|error| AppError::internal(format!("加入轮询任务失败: {error}")))?;
        Ok(())
    }

    async fn get_snapshot(&self, streamer_id: &str) -> Result<Option<PollSnapshot>, AppError> {
        let mut conn = self.connection().await?;
        let raw: Option<String> = conn
            .get(Self::snap_key(streamer_id))
            .await
            .map_err(|error| AppError::internal(format!("读取快照失败: {error}")))?;
        raw.map(|json| {
            serde_json::from_str(&json)
                .map_err(|error| AppError::internal(format!("快照反序列化失败: {error}")))
        })
        .transpose()
    }

    async fn save_snapshot(&self, snapshot: &PollSnapshot) -> Result<(), AppError> {
        let json = serde_json::to_string(snapshot)
            .map_err(|error| AppError::internal(format!("快照序列化失败: {error}")))?;
        let mut conn = self.connection().await?;
        let _: () = conn
            .set_ex(Self::snap_key(&snapshot.streamer_id), json, SNAP_TTL_SECS)
            .await
            .map_err(|error| AppError::internal(format!("写入快照失败: {error}")))?;
        Ok(())
    }

    async fn invalidate_snapshot(&self, streamer_id: &str) -> Result<(), AppError> {
        let mut conn = self.connection().await?;
        let _: () = conn
            .del(Self::snap_key(streamer_id))
            .await
            .map_err(|error| AppError::internal(format!("删除快照失败: {error}")))?;
        Ok(())
    }
}
