//! 应用配置：统一从环境变量读取

use std::env;

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 日志级别
    pub rust_log: String,
    /// PostgreSQL 连接串
    pub database_url: String,
    /// Redis 连接串
    pub redis_url: String,
    /// JWT 签名密钥
    pub jwt_secret: String,
    /// Token 有效期（小时）
    pub jwt_expires_hours: i64,
    /// 短信验证码有效期（秒）
    pub sms_code_expire_in: u64,
    /// 短信重发冷却（秒）
    pub sms_code_resend_cooldown: u64,
    /// 固定验证码（mock 阶段联调用；为空则随机生成）
    pub mock_fixed_code: Option<String>,
}

impl AppConfig {
    /// 从环境变量加载，缺失项使用默认值
    pub fn from_env() -> Self {
        Self {
            host: env_or("SERVER_HOST", "0.0.0.0"),
            port: env_or_parse("SERVER_PORT", 8080),
            rust_log: env_or("RUST_LOG", "si_land_server=debug,tower_http=debug"),
            database_url: env_or(
                "DATABASE_URL",
                "postgres://siland:siland123@localhost:5432/siland",
            ),
            redis_url: env_or("REDIS_URL", "redis://:siland123@localhost:6379/0"),
            jwt_secret: env_or("JWT_SECRET", "please-change-me-in-production"),
            jwt_expires_hours: env_or_parse("JWT_EXPIRES_HOURS", 168),
            sms_code_expire_in: env_or_parse("SMS_CODE_EXPIRE_IN", 300),
            sms_code_resend_cooldown: env_or_parse("SMS_CODE_RESEND_COOLDOWN", 60),
            mock_fixed_code: env::var("MOCK_FIXED_CODE")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string()),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
