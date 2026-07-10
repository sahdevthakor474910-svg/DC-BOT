-- Migration 005: Porn video channel (RedTube API) + deduplication table
ALTER TABLE guild_config ADD COLUMN porn_video_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_porn_videos (
    guild_id   TEXT     NOT NULL,
    video_id   TEXT     NOT NULL,
    seen_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, video_id)
);
