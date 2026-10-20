ALTER TABLE guild_config ADD COLUMN indian_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_indian (
    guild_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    seen_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (guild_id, video_id)
);
