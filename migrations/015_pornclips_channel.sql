ALTER TABLE guild_config ADD COLUMN pornclips_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_pornclips (
    guild_id TEXT NOT NULL,
    item_id  TEXT NOT NULL,
    seen_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (guild_id, item_id)
);
