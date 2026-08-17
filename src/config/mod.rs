//! 应用配置：统一从环境变量加载
//!
//! 敏感项（DATABASE_URL / JWT_SECRET）无默认值，缺失时启动失败，
//! 避免生产环境用弱密码裸奔。

pub mod constants;
pub mod env;

use crate::error::AppError;

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub sms: SmsConfig,
    pub admin: AdminConfig,
    pub douyin: DouyinConfig,
    pub poll: PollConfig,
    pub push: PushConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    pub db_max_connections: u32,
    /// 允许跨域访问的前端 Origin 白名单。
    pub cors_allowed_origins: Vec<String>,
    pub ws_ticket_expires_secs: i64,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_hours: i64,
}

#[derive(Debug, Clone)]
pub struct SmsConfig {
    pub code_expire_in: u64,
    pub resend_cooldown: u64,
    pub code_length: usize,
    /// 固定验证码（mock 联调用）；为空则随机生成
    pub mock_fixed_code: Option<String>,
    pub default_nickname: String,
    pub max_sends_per_hour: i32,
    pub max_verify_attempts: i32,
}

/// 管理员后台账号配置（内置单管理员，用户名密码登录）
#[derive(Clone)]
pub struct AdminConfig {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for AdminConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminConfig")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct DouyinConfig {
    pub http_timeout_secs: u64,
    pub max_redirects: usize,
    pub user_agent: String,
    pub referer: String,
    pub ttwid_register_url: String,
    pub enter_api_url: String,
    pub web_rid_base_url: String,
}

/// 轮询调度配置：数据库驱动，单调度循环 + 短期并发任务
#[derive(Debug, Clone)]
pub struct PollConfig {
    /// 调度循环间隔（秒）。2-3 秒推荐，过短会增加 DB 压力
    pub loop_interval_secs: u64,
    /// 单次批量抓取数量
    pub batch_size: i64,
    /// 抖音接口全局最大并发请求数
    pub max_concurrency: usize,
    /// 单次检测请求超时（秒）
    pub check_timeout_secs: u64,
    /// 在播时轮询间隔（秒）—— 短，及时检测下播
    pub interval_live_secs: i64,
    /// 未播时轮询间隔（秒）—— 常规
    pub interval_idle_secs: i64,
    /// 请求失败后基础退避（秒）
    pub backoff_base_secs: i64,
    /// 退避指数（每次失败间隔翻倍上限）
    pub backoff_max_secs: i64,
    /// 抖动范围（秒），next_poll += rand(0, jitter)
    pub jitter_secs: i64,
}

/// 推送配置
#[derive(Debug, Clone)]
pub struct PushConfig {
    /// 默认推送通道：bark / apns
    pub default_channel: String,
    /// Bark 服务地址（如 https://api.day.app/{key}）
    pub bark_base_url: String,
    /// 请求超时（秒）
    pub timeout_secs: u64,
}

/// Kafka 配置（通知消息队列）
#[derive(Debug, Clone)]
pub struct KafkaConfig {
    /// broker 地址列表，逗号分隔（如 localhost:9092）
    pub brokers: String,
    /// 开播通知 topic
    pub notification_topic: String,
    /// 通知 Worker 消费者组
    pub group_id: String,
}

impl AppConfig {
    /// 从环境变量加载，必填项缺失返回错误
    pub fn load() -> Result<Self, AppError> {
        let app_env = env::or("APP_ENV", "development");
        let cors_allowed_origins = env::csv("CORS_ALLOWED_ORIGINS");
        if app_env == "production" && cors_allowed_origins.is_empty() {
            return Err(AppError::internal("生产环境必须配置 CORS_ALLOWED_ORIGINS"));
        }
        Ok(Self {
            server: ServerConfig {
                host: env::or("SERVER_HOST", "0.0.0.0"),
                port: env::parse_or("SERVER_PORT", 8080),
                rust_log: env::or("RUST_LOG", "si_land_server=debug,tower_http=debug"),
                db_max_connections: env::parse_or("DB_MAX_CONNECTIONS", 8),
                cors_allowed_origins: if cors_allowed_origins.is_empty() {
                    vec![
                        "http://localhost:5173".to_string(),
                        "http://127.0.0.1:5173".to_string(),
                    ]
                } else {
                    cors_allowed_origins
                },
                ws_ticket_expires_secs: env::parse_or("WS_TICKET_EXPIRES_SECS", 60),
            },
            database: DatabaseConfig {
                url: env::required("DATABASE_URL")?,
            },
            redis: RedisConfig {
                url: env::required("REDIS_URL")?,
            },
            jwt: JwtConfig {
                secret: env::required("JWT_SECRET")?,
                expires_hours: env::parse_or("JWT_EXPIRES_HOURS", 168),
            },
            sms: SmsConfig {
                code_expire_in: env::parse_or("SMS_CODE_EXPIRE_IN", 300),
                resend_cooldown: env::parse_or("SMS_CODE_RESEND_COOLDOWN", 60),
                code_length: env::parse_or("SMS_CODE_LENGTH", 6),
                mock_fixed_code: env::optional("MOCK_FIXED_CODE"),
                default_nickname: env::or("DEFAULT_USER_NICKNAME", "矽澜用户"),
                max_sends_per_hour: env::parse_or("SMS_MAX_SENDS_PER_HOUR", 5),
                max_verify_attempts: env::parse_or("SMS_MAX_VERIFY_ATTEMPTS", 5),
            },
            admin: AdminConfig {
                username: env::required("ADMIN_USERNAME")?,
                password: env::required("ADMIN_PASSWORD")?,
            },
            douyin: DouyinConfig {
                http_timeout_secs: env::parse_or("DOUYIN_HTTP_TIMEOUT_SECS", 10),
                max_redirects: env::parse_or("DOUYIN_MAX_REDIRECTS", 3),
                user_agent: env::or(
                    "DOUYIN_USER_AGENT",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                     AppleWebKit/537.36 (KHTML, like Gecko) \
                     Chrome/109.0.0.0 Safari/537.36",
                ),
                referer: env::or("DOUYIN_REFERER", "https://live.douyin.com/"),
                ttwid_register_url: env::or(
                    "DOUYIN_TTWID_REGISTER_URL",
                    "https://ttwid.bytedance.com/ttwid/union/register/",
                ),
                enter_api_url: env::or(
                    "DOUYIN_ENTER_API_URL",
                    "https://live.douyin.com/webcast/room/web/enter/",
                ),
                web_rid_base_url: env::or("DOUYIN_WEB_RID_BASE_URL", "https://live.douyin.com/"),
            },
            poll: PollConfig {
                loop_interval_secs: env::parse_or("POLL_LOOP_INTERVAL_SECS", 3),
                batch_size: env::parse_or("POLL_BATCH_SIZE", 20),
                max_concurrency: env::parse_or("POLL_MAX_CONCURRENCY", 4),
                check_timeout_secs: env::parse_or("POLL_CHECK_TIMEOUT_SECS", 10),
                interval_live_secs: env::parse_or("POLL_INTERVAL_LIVE_SECS", 60),
                interval_idle_secs: env::parse_or("POLL_INTERVAL_IDLE_SECS", 60),
                backoff_base_secs: env::parse_or("POLL_BACKOFF_BASE_SECS", 60),
                backoff_max_secs: env::parse_or("POLL_BACKOFF_MAX_SECS", 1800),
                jitter_secs: env::parse_or("POLL_JITTER_SECS", 15),
            },
            push: PushConfig {
                default_channel: env::or("PUSH_DEFAULT_CHANNEL", "bark"),
                bark_base_url: env::or("PUSH_BARK_BASE_URL", "https://api.day.app"),
                timeout_secs: env::parse_or("PUSH_TIMEOUT_SECS", 5),
            },
            kafka: KafkaConfig {
                brokers: env::or("KAFKA_BROKERS", "localhost:9092"),
                notification_topic: env::or("KAFKA_NOTIFICATION_TOPIC", "notifications"),
                group_id: env::or("KAFKA_GROUP_ID", "siland-notifications"),
            },
        })
    }
}
