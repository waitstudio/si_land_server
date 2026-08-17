-- 0002 推送通道 / 开播通知
-- 权威迁移文件，启动时由 src/services/db/migrations.rs 按文件名顺序幂等执行
-- （轮询调度已迁移 Redis ZSet，不再建调度表）

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

-- live_notices：开播通知记录（用户消息页数据源）
CREATE TABLE IF NOT EXISTS live_notices (
    id                VARCHAR(64)  PRIMARY KEY,
    user_id           VARCHAR(64)  NOT NULL,
    streamer_id       VARCHAR(64)  NOT NULL,
    streamer_nickname VARCHAR(128) NOT NULL DEFAULT '',
    title             VARCHAR(128) NOT NULL DEFAULT '',
    body              TEXT         NOT NULL DEFAULT '',
    live_started_at   BIGINT,
    created_at        BIGINT       NOT NULL,
    read              BOOLEAN      NOT NULL DEFAULT FALSE,
    CONSTRAINT fk_notice_user    FOREIGN KEY (user_id)
        REFERENCES users(user_id) ON DELETE CASCADE,
    CONSTRAINT fk_notice_streamer FOREIGN KEY (streamer_id)
        REFERENCES streamers(id) ON DELETE CASCADE
);

-- 分页查询主索引（按用户 + 时间倒序）
CREATE INDEX IF NOT EXISTS idx_notices_user_time
    ON live_notices (user_id, created_at DESC);

-- 未读计数部分索引（只索引未读行）
CREATE INDEX IF NOT EXISTS idx_notices_unread
    ON live_notices (user_id) WHERE read = FALSE;
