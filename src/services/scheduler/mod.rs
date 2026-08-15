//! 轮询调度模块
//!
//! 数据库驱动调度模式：
//! - 单常驻循环查询到期任务
//! - 短期异步任务执行检测
//! - 自适应间隔 + 随机抖动
//! - 多实例部署用 FOR UPDATE SKIP LOCKED 避免任务重复

pub mod adaptive_interval;
pub mod poll_scheduler;
pub mod poll_store;

pub use poll_scheduler::PollScheduler;
pub use poll_store::{PgPollStore, PollStore};
