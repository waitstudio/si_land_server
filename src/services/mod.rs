//! 基础设施服务层
//!
//! 用 trait 抽象外部依赖（数据库、抖音号解析、开播检测、短信通道、轮询调度、推送），
//! 业务代码只依赖 trait，替换实现无需改动。

pub mod code_store;
pub mod db;
pub mod douyin;
pub mod feedback_store;
pub mod jwt;
pub mod notice_store;
pub mod notification_outbox;
pub mod outbox_worker;
pub mod push;
pub mod scheduler;
pub mod sms_provider;
pub mod streamer_wish_store;
pub mod subscription_store;
pub mod user_store;
pub mod ws_hub;
pub mod ws_ticket_store;
