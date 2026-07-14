-- Migration 009: X / Twitter update channel (Nitter RSS feed)
ALTER TABLE guild_config ADD COLUMN twitter_channel_id TEXT;

CREATE TABLE IF NOT EXISTS seen_tweets (
    guild_id  TEXT NOT NULL,
    tweet_id  TEXT NOT NULL,
    seen_at   TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (guild_id, tweet_id)
);
