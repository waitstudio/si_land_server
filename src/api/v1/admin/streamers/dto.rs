//! 管理员主播收录模块的请求 / 响应 DTO

use serde::Deserialize;

/// 收录主播请求
#[derive(Debug, Deserialize)]
pub struct AddStreamerRequest {
    /// 抖音号（用户自定义短号，非链接 / 分享口令）
    pub douyin_id: String,
}
