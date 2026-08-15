//! 管理员认证模块（对接管理后台）
//!
//! 内置管理员账号（env 配置 ADMIN_USERNAME / ADMIN_PASSWORD），
//! 用户名密码登录签发 JWT，token sub 固定为管理员标识，
//! 由 `admin_guard` 中间件校验。
//!
//! 同时提供管理后台框架（vue-vben-admin）所需的配套接口：
//! 用户信息 / 权限码 / 菜单 / 登出。

mod dto;
mod handler;
mod service;

pub use handler::{codes, login, logout, menus, user_info};
