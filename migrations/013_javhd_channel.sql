ALTER TABLE guild_config ADD COLUMN javhd_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_javhd (
    guild_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    seen_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, video_id)
);
