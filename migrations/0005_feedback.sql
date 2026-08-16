-- 0005 问题反馈收集表
-- 启动时由 src/services/db/migrations.rs 幂等执行

-- 收集用户提交的 BUG、功能建议等反馈，运营查表跟进。
CREATE TABLE IF NOT EXISTS feedbacks (
    id         VARCHAR(64) PRIMARY KEY,    -- fb_ + ULID
    user_id    VARCHAR(64) NOT NULL,
    content    VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_feedback_user
        FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

-- 按用户 + 时间倒序（运营按提交时间查看）
CREATE INDEX IF NOT EXISTS idx_feedbacks_user_time
    ON feedbacks (user_id, created_at DESC);
