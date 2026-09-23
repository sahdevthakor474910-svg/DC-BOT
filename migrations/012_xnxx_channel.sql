ALTER TABLE guild_config ADD COLUMN xnxx_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_xnxx (
    guild_id TEXT NOT NULL,
    item_id  TEXT NOT NULL,
    seen_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (guild_id, item_id)
);
