ALTER TABLE guild_config ADD COLUMN hanime_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_hanime (
    guild_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    seen_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, video_id)
);
