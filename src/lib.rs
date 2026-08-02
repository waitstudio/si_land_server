//! si_land_server 库入口
//!
//! 把所有模块通过 lib 暴露，便于集成测试与未来拆分。

pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod middleware;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;
pub mod utils;

pub use config::AppConfig;
pub use error::AppError;
pub use response::{ApiResponse, BizCode};
pub use state::{build_state, AppState};
