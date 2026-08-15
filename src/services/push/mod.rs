//! 推送模块：多通道推送抽象
//!
//! - [PushProvider] trait 隔离具体推送通道
//! - [PushTokenStore] 管理用户推送凭证
//! - 业务层只依赖这两个抽象，新增通道只需实现 trait

pub mod bark;
pub mod provider;
pub mod token_store;

pub use provider::{PushMessage, PushProvider};
pub use token_store::{PgPushTokenStore, PushTokenStore};
