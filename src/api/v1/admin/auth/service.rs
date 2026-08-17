//! 管理员认证业务编排

use crate::config::constants;
use crate::error::AppError;
use crate::response::BizCode;
use crate::services::jwt;
use crate::state::AppState;

use super::dto::{AdminLoginResponse, AdminUserInfo};

pub struct AdminAuthService;

impl AdminAuthService {
    /// 管理员用户名密码登录
    ///
    /// 与 env 配置的内置账号比对，成功签发 sub 为管理员标识的 JWT。
    /// 该 token 只能访问 /api/v1/admin/* 下的受保护接口（admin_guard 校验）。
    pub async fn login(
        state: &AppState,
        username: &str,
        password: &str,
    ) -> Result<AdminLoginResponse, AppError> {
        let admin = &state.config.admin;
        if username != admin.username || password != admin.password {
            return Err(AppError::new(
                BizCode::Unauthorized,
                "管理员用户名或密码错误",
            ));
        }
        let token = jwt::sign(
            constants::ADMIN_SUBJECT,
            &state.config.jwt.secret,
            state.config.jwt.expires_hours,
        )?;
        Ok(AdminLoginResponse {
            access_token: token,
        })
    }

    /// 管理员用户信息
    pub fn user_info() -> AdminUserInfo {
        AdminUserInfo {
            user_id: constants::ADMIN_SUBJECT.to_string(),
            username: "admin".to_string(),
            real_name: "管理员".to_string(),
            roles: vec!["super".to_string()],
            avatar: String::new(),
            home_path: "/streamer".to_string(),
        }
    }
}
