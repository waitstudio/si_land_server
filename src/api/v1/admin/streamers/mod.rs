//! 管理员主播收录模块
//!
//! 管理员输入抖音号收录主播到热门列表（原 App 端"手动订阅"逻辑迁移至此）。

mod dto;
mod handler;
mod service;

pub use handler::{add_streamer, list_streamers};
