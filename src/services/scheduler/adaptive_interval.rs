//! 自适应轮询间隔策略
//!
//! 规则：
//! - 直播中 → 短间隔（及时检测下播）
//! - 未开播 → 常规间隔
//! - 连续失败 → 指数退避（base * 2^fail，封顶 max）
//! - 所有间隔 + 随机抖动（0..=jitter_secs），规避风控

use rand::Rng;

use crate::config::PollConfig;

/// 计算下次轮询的 next_poll_at（Unix 秒）
///
/// 参数：
/// - `now` 当前时间戳
/// - `is_live` 本次检测是否在播
/// - `fail_count` 连续失败次数（成功时为 0）
pub fn next_poll_at(now: i64, is_live: bool, fail_count: i32, cfg: &PollConfig) -> i64 {
    let base = if fail_count > 0 {
        backoff(fail_count, cfg)
    } else if is_live {
        cfg.interval_live_secs
    } else {
        cfg.interval_idle_secs
    };

    let jitter = if cfg.jitter_secs > 0 {
        rand::thread_rng().gen_range(0..=cfg.jitter_secs)
    } else {
        0
    };

    now + base + jitter
}

/// 指数退避：base * 2^(fail-1)，封顶 max
fn backoff(fail_count: i32, cfg: &PollConfig) -> i64 {
    let shift = (fail_count - 1).max(0).min(10) as u32;
    let scaled = cfg.backoff_base_secs.saturating_mul(2_i64.pow(shift));
    scaled.min(cfg.backoff_max_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PollConfig {
        PollConfig {
            loop_interval_secs: 3,
            batch_size: 20,
            max_concurrency: 4,
            check_timeout_secs: 10,
            interval_live_secs: 60,
            interval_idle_secs: 300,
            backoff_base_secs: 60,
            backoff_max_secs: 1800,
            jitter_secs: 0, // 测试时关掉抖动
        }
    }

    #[test]
    fn live_uses_short_interval() {
        let next = next_poll_at(1000, true, 0, &cfg());
        assert_eq!(next, 1000 + 60);
    }

    #[test]
    fn idle_uses_normal_interval() {
        let next = next_poll_at(1000, false, 0, &cfg());
        assert_eq!(next, 1000 + 300);
    }

    #[test]
    fn fail_triggers_backoff() {
        let next = next_poll_at(1000, false, 1, &cfg());
        assert_eq!(next, 1000 + 60);
        let next = next_poll_at(1000, false, 2, &cfg());
        assert_eq!(next, 1000 + 120);
    }

    #[test]
    fn backoff_capped_at_max() {
        let next = next_poll_at(1000, false, 10, &cfg());
        assert_eq!(next, 1000 + 1800);
    }
}
