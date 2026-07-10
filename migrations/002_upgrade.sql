-- Bot v2: Extended guild config + new deduplication tables
-- SQLite ALTER TABLE does not support multiple columns per statement.
-- The migration runner in schema.rs silently skips "duplicate column name" errors,
-- so running this file more than once is safe.

ALTER TABLE guild_config ADD COLUMN brainrot_channel_id     TEXT;
ALTER TABLE guild_config ADD COLUMN shitposting_channel_id  TEXT;
ALTER TABLE guild_config ADD COLUMN instagram_channel_id    TEXT;
ALTER TABLE guild_config ADD COLUMN news_channel_id         TEXT;
ALTER TABLE guild_config ADD COLUMN free_games_channel_id   TEXT;
ALTER TABLE guild_config ADD COLUMN nsfw_channel_id         TEXT;
ALTER TABLE guild_config ADD COLUMN auto_react_enabled      INTEGER NOT NULL DEFAULT 1;

CREATE TABLE IF NOT EXISTS seen_news (
    guild_id    TEXT     NOT NULL,
    article_id  TEXT     NOT NULL,
    seen_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, article_id)
);

CREATE TABLE IF NOT EXISTS seen_giveaways (
    guild_id     TEXT     NOT NULL,
    giveaway_id  TEXT     NOT NULL,
    seen_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (guild_id, giveaway_id)
);

CREATE TABLE IF NOT EXISTS channel_emojis (
    guild_id   TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    emoji      TEXT NOT NULL,
    PRIMARY KEY (guild_id, channel_id, emoji)
);

CREATE TABLE IF NOT EXISTS user_emojis (
    guild_id TEXT NOT NULL,
    user_id  TEXT NOT NULL,
    emoji    TEXT NOT NULL,
    PRIMARY KEY (guild_id, user_id, emoji)
);
