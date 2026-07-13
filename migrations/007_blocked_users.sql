-- Migration 007: per-guild user blocklist
CREATE TABLE IF NOT EXISTS blocked_users (
    guild_id  TEXT NOT NULL,
    user_id   TEXT NOT NULL,
    blocked_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (guild_id, user_id)
);
