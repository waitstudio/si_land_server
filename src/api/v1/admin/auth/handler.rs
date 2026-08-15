//! 管理员认证 handler

use axum::{extract::State, Json};

use crate::error::AppError;
use crate::response::ApiResponse;
use crate::state::AppState;

use super::dto::{AdminLoginRequest, AdminLoginResponse, AdminUserInfo};
use super::service::AdminAuthService;

/// POST /api/v1/admin/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<AdminLoginRequest>,
) -> Result<Json<ApiResponse<AdminLoginResponse>>, AppError> {
    let result = AdminAuthService::login(&state, &req.username, &req.password).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/admin/user/info
///
/// 管理后台登录后拉取用户信息（已过 admin_guard，必为管理员）。
pub async fn user_info() -> Result<Json<ApiResponse<AdminUserInfo>>, AppError> {
    Ok(Json(ApiResponse::success(AdminAuthService::user_info())))
}

/// GET /api/v1/admin/auth/codes
///
/// 管理后台按钮级权限码，当前无细分权限，返回空列表。
pub async fn codes() -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
    Ok(Json(ApiResponse::success(Vec::new())))
}

/// GET /api/v1/admin/menu/all
///
/// 管理后台菜单接口。菜单由前端路由模块定义（frontend accessMode），返回空列表。
pub async fn menus() -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    Ok(Json(ApiResponse::success(Vec::new())))
}

/// POST /api/v1/admin/auth/logout
///
/// JWT 无状态，登出由前端清除 token 即可，这里返回成功。
pub async fn logout() -> Result<Json<ApiResponse<()>>, AppError> {
    Ok(Json(ApiResponse::success(())))
}
