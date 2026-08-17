-- 0004 问题反馈收集表
-- 权威迁移文件，启动时由 src/services/db/migrations.rs 按文件名顺序幂等执行

CREATE TABLE IF NOT EXISTS feedbacks (
    id         VARCHAR(64)  PRIMARY KEY,
    user_id    VARCHAR(64)  NOT NULL,
    content    VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_feedback_user
        FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feedbacks_user_time
    ON feedbacks (user_id, created_at DESC);
