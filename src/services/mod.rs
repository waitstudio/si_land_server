//! 基础设施服务层
//!
//! 用 trait 抽象外部依赖（短信通道、验证码存储），
//! mock 阶段提供内存实现，后续可替换为 Redis / 短信服务商实现，业务代码无需改动。

pub mod code_store;
pub mod sms_provider;
