-- 初始建表迁移
-- 启动时由 src/services/db/migrations.rs 幂等执行（CREATE TABLE IF NOT EXISTS）
-- 本文件用于审计与版本管理

-- users 表：用户基础信息
CREATE TABLE IF NOT EXISTS users (
    user_id        VARCHAR(64)  PRIMARY KEY,
    phone          VARCHAR(20)  UNIQUE NOT NULL,
    nickname       VARCHAR(64)  NOT NULL DEFAULT '',
    avatar         TEXT         NOT NULL DEFAULT '',
    status         SMALLINT     NOT NULL DEFAULT 1,
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_login_at  TIMESTAMPTZ
);

-- streamers 表：主播信息（跨用户共享，按 sec_uid 去重）
CREATE TABLE IF NOT EXISTS streamers (
    id              VARCHAR(64)  PRIMARY KEY,
    sec_uid         VARCHAR(128) UNIQUE NOT NULL,
    douyin_id       VARCHAR(64)  NOT NULL DEFAULT '',
    nickname        VARCHAR(128) NOT NULL DEFAULT '',
    avatar          TEXT         NOT NULL DEFAULT '',
    live            BOOLEAN      NOT NULL DEFAULT FALSE,
    live_started_at BIGINT,
    popularity      BIGINT       NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
ALTER TABLE streamers ADD COLUMN IF NOT EXISTS popularity BIGINT NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_streamers_popularity ON streamers (popularity DESC, updated_at DESC);

-- subscriptions 表：用户-主播订阅关系
CREATE TABLE IF NOT EXISTS subscriptions (
    user_id        VARCHAR(64) NOT NULL,
    streamer_id    VARCHAR(64) NOT NULL,
    subscribed_at  BIGINT      NOT NULL,
    PRIMARY KEY (user_id, streamer_id),
    FOREIGN KEY (user_id)     REFERENCES users(user_id)     ON DELETE CASCADE,
    FOREIGN KEY (streamer_id) REFERENCES streamers(id)     ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_subscriptions_streamer ON subscriptions (streamer_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_user     ON subscriptions (user_id, subscribed_at DESC);

-- updated_at 触发器
CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_users_updated ON users;
CREATE TRIGGER trg_users_updated
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

DROP TRIGGER IF EXISTS trg_streamers_updated ON streamers;
CREATE TRIGGER trg_streamers_updated
    BEFORE UPDATE ON streamers
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
