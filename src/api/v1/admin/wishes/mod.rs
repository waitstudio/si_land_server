//! 管理员想看意愿查询模块
//!
//! 展示 App 端用户提交的"想看"主播抖音号及计数，
//! 供运营决策是否收录。

mod dto;
mod handler;
mod service;

pub use handler::list_wishes;
