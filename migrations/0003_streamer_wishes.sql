-- 0003 主播想看意愿收集表
-- 权威迁移文件，启动时由 src/services/db/migrations.rs 按文件名顺序幂等执行
-- 按 douyin_id 去重累计想看数，运营据此决定是否收录该主播

CREATE TABLE IF NOT EXISTS streamer_wishes (
    douyin_id   VARCHAR(64) PRIMARY KEY,
    want_count  BIGINT      NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_streamer_wishes_count
    ON streamer_wishes (want_count DESC);
