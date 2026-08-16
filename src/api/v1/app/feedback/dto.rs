//! 问题反馈模块的请求 / 响应 DTO

use serde::{Deserialize, Serialize};

/// 提交反馈请求
#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    /// 反馈内容（BUG / 功能建议等）
    pub content: String,
}

/// 提交反馈响应
#[derive(Debug, Serialize)]
pub struct FeedbackResponse {
    /// 落库后的反馈 ID
    pub id: String,
}
