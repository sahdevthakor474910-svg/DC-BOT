CREATE TABLE IF NOT EXISTS menace_targets (
    guild_id TEXT NOT NULL,
    user_id  TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (guild_id, user_id)
);
