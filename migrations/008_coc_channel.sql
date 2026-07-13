-- Migration 008: Clash of Clans update channel + seen_coc dedup table

ALTER TABLE guild_config ADD COLUMN coc_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_coc (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id    TEXT    NOT NULL,
    item_id     TEXT    NOT NULL,
    seen_at     INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    UNIQUE (guild_id, item_id)
);
