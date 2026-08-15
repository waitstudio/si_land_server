//! 基础设施服务层
//!
//! 用 trait 抽象外部依赖（数据库、抖音号解析、开播检测、短信通道、轮询调度、推送），
//! 业务代码只依赖 trait，替换实现无需改动。

pub mod code_store;
pub mod db;
pub mod douyin;
pub mod jwt;
pub mod push;
pub mod scheduler;
pub mod sms_provider;
pub mod subscription_store;
pub mod user_store;
