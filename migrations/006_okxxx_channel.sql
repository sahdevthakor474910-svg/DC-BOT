-- Migration 006: ok.xxx channel + deduplication table
ALTER TABLE guild_config ADD COLUMN okxxx_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_okxxx (
    guild_id   TEXT NOT NULL,
    video_id   TEXT NOT NULL,
    seen_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (guild_id, video_id)
);
