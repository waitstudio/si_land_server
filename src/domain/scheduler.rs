//! 轮询调度领域模型

use sqlx::FromRow;

/// 单个主播的轮询任务记录
#[derive(Debug, Clone, FromRow)]
pub struct PollTask {
    pub streamer_id: String,
    pub poll_enabled: bool,
    pub next_poll_at: i64,
    /// 0=未知 1=在播 2=未播
    pub last_status: i16,
    pub last_poll_at: Option<i64>,
    pub fail_count: i32,
}

impl PollTask {
    pub fn is_live(&self) -> bool {
        self.last_status == 1
    }
}
