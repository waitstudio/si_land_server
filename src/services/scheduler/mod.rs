//! 轮询调度模块
//!
//! Redis 驱动调度模式（取出-执行-放回）：
//! - 单常驻循环从 ZSet 领取到期任务（原子抢占）
//! - 短期异步任务执行检测，快照对比仅在变更时写 DB
//! - 自适应间隔 + 随机抖动
//! - 多实例部署天然无争抢，实例崩溃由 inflight 兜底回收

pub mod adaptive_interval;
pub mod poll_scheduler;
pub mod redis_poll_store;

pub use poll_scheduler::PollScheduler;
pub use redis_poll_store::{PollSnapshot, PollStore, RedisPollStore};
