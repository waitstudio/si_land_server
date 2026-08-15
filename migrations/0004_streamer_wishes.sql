-- 0004 主播想看意愿收集表
-- 启动时由 src/services/db/migrations.rs 幂等执行

-- 收集用户想看但尚未收录的主播，按抖音号去重，记录想看计数。
-- 运营根据计数决定是否将该主播加入 streamers 表。
CREATE TABLE IF NOT EXISTS streamer_wishes (
    douyin_id   VARCHAR(64) PRIMARY KEY,    -- 抖音号，去重 key
    want_count  BIGINT      NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 按想看计数降序，运营查表时优先看最想看的
CREATE INDEX IF NOT EXISTS idx_streamer_wishes_count
    ON streamer_wishes (want_count DESC);
