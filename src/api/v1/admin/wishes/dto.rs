//! 管理员想看意愿查询模块的请求 / 响应 DTO

use serde::Deserialize;

/// 想看意愿列表查询参数
#[derive(Debug, Deserialize)]
pub struct WishListQuery {
    pub limit: Option<i64>,
}
