-- 0002 轮询调度与推送通道
-- 启动时由 src/services/db/migrations.rs 幂等执行

-- streamer_poll_tasks：每个主播一条调度记录
-- 与 streamers 表 1:1，通过 FK 级联删除
CREATE TABLE IF NOT EXISTS streamer_poll_tasks (
    streamer_id    VARCHAR(64) PRIMARY KEY,
    poll_enabled   BOOLEAN     NOT NULL DEFAULT TRUE,
    next_poll_at   BIGINT      NOT NULL DEFAULT 0,
    last_status    SMALLINT    NOT NULL DEFAULT 0,   -- 0=未知 1=在播 2=未播
    last_poll_at   BIGINT,
    fail_count     INT         NOT NULL DEFAULT 0,
    CONSTRAINT fk_poll_streamer
        FOREIGN KEY (streamer_id) REFERENCES streamers(id) ON DELETE CASCADE
);

-- 调度核心索引：按 next_poll_at 拉取到期任务，仅含启用项
CREATE INDEX IF NOT EXISTS idx_poll_next
    ON streamer_poll_tasks (next_poll_at)
    WHERE poll_enabled = TRUE;

-- user_push_tokens：用户推送凭证（一个用户可绑定多通道）
CREATE TABLE IF NOT EXISTS user_push_tokens (
    user_id    VARCHAR(64) NOT NULL,
    channel    VARCHAR(16) NOT NULL,            -- 'bark' / 'apns'
    token      TEXT        NOT NULL,
    enabled    BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, channel),
    CONSTRAINT fk_push_user
        FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON user_push_tokens (user_id);
