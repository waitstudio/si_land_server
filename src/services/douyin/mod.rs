//! 抖音相关基础设施
//!
//! 基于 `webcast/room/web/enter/` 官方 JSON 接口。
//! enter 接口同时返回 sec_uid / nickname / avatar 与开播状态，
//! 故 [live_checker::HttpLiveChecker] 与 [streamer_resolver::HttpStreamerResolver]
//! 共享同一个 [client::DouyinEnterClient] 实例（Arc），避免重复请求与 ttwid 注册。
//!
//! # 反爬说明
//!
//! 该接口需 `ttwid`，但无需 X-Bogus / a_bogus 签名，相对稳定。
//! ttwid 失效时会自动重新注册并重试一次。生产环境如仍遇风控，
//! 建议配合代理 IP 池 + 定期刷新 ttwid。

pub mod client;
pub mod enter_parser;
pub mod live_checker;
pub mod streamer_resolver;
