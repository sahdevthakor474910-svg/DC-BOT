-- Migration 004: Add JAV channel + seen_jav deduplication table
ALTER TABLE guild_config ADD COLUMN jav_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_jav (
    guild_id   TEXT     NOT NULL,
    content_id TEXT     NOT NULL,
    seen_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, content_id)
);
