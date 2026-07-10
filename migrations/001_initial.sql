-- Guild-level configuration (one row per guild)
CREATE TABLE IF NOT EXISTS guild_config (
    guild_id              TEXT    PRIMARY KEY,
    meme_channel_id       TEXT,
    posting_interval_secs INTEGER NOT NULL DEFAULT 300
);

-- Channels where every message gets auto-reacted
CREATE TABLE IF NOT EXISTS reaction_channels (
    guild_id   TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    PRIMARY KEY (guild_id, channel_id)
);

-- Users whose messages always get auto-reacted
CREATE TABLE IF NOT EXISTS reaction_users (
    guild_id TEXT NOT NULL,
    user_id  TEXT NOT NULL,
    PRIMARY KEY (guild_id, user_id)
);

-- Custom emoji list per guild
CREATE TABLE IF NOT EXISTS reaction_emojis (
    guild_id TEXT NOT NULL,
    emoji    TEXT NOT NULL,
    PRIMARY KEY (guild_id, emoji)
);

-- Seen Reddit post IDs for deduplication (per guild)
CREATE TABLE IF NOT EXISTS seen_posts (
    guild_id  TEXT     NOT NULL,
    post_id   TEXT     NOT NULL,
    posted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, post_id)
);

-- Prune seen posts older than 30 days to prevent unbounded growth
CREATE INDEX IF NOT EXISTS idx_seen_posts_posted_at ON seen_posts (posted_at);
