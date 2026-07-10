use anyhow::Result;
use sqlx::{Row, SqlitePool};

// ────────────────────────────────────────────────────────────────────────────
// Structs returned by queries
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GuildConfig {
    pub guild_id: String,
    pub meme_channel_id: Option<String>,
    pub posting_interval_secs: i64,
    pub brainrot_channel_id: Option<String>,
    pub shitposting_channel_id: Option<String>,
    pub instagram_channel_id: Option<String>,
    pub news_channel_id: Option<String>,
    pub free_games_channel_id: Option<String>,
    pub nsfw_channel_id: Option<String>,  // added via migration 003
    pub rule34_channel_id: Option<String>,
    pub porn_channel_id: Option<String>,
    pub hentai_channel_id: Option<String>,
    pub auto_react_enabled: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// Guild config
// ────────────────────────────────────────────────────────────────────────────

/// Ensure a row exists for the guild and return it.
pub async fn get_or_create_guild(db: &SqlitePool, guild_id: &str) -> Result<GuildConfig> {
    sqlx::query(
        "INSERT OR IGNORE INTO guild_config (guild_id, posting_interval_secs, auto_react_enabled) VALUES (?, 300, 1)",
    )
    .bind(guild_id)
    .execute(db)
    .await?;

    let row = sqlx::query(
        "SELECT guild_id, meme_channel_id, posting_interval_secs, \
                brainrot_channel_id, shitposting_channel_id, instagram_channel_id, \
                news_channel_id, free_games_channel_id, nsfw_channel_id, rule34_channel_id, \
                porn_channel_id, hentai_channel_id, auto_react_enabled \
         FROM guild_config WHERE guild_id = ?",
    )
    .bind(guild_id)
    .fetch_one(db)
    .await?;

    Ok(GuildConfig {
        guild_id: row.get("guild_id"),
        meme_channel_id: row.get("meme_channel_id"),
        posting_interval_secs: row.get("posting_interval_secs"),
        brainrot_channel_id: row.get("brainrot_channel_id"),
        shitposting_channel_id: row.get("shitposting_channel_id"),
        instagram_channel_id: row.get("instagram_channel_id"),
        news_channel_id: row.get("news_channel_id"),
        free_games_channel_id: row.get("free_games_channel_id"),
        nsfw_channel_id: row.get("nsfw_channel_id"),
        rule34_channel_id: row.get("rule34_channel_id"),
        porn_channel_id: row.get("porn_channel_id"),
        hentai_channel_id: row.get("hentai_channel_id"),
        auto_react_enabled: row.get::<i64, _>("auto_react_enabled") != 0,
    })
}

pub async fn set_meme_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, meme_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET meme_channel_id = excluded.meme_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_brainrot_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, brainrot_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET brainrot_channel_id = excluded.brainrot_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_shitposting_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, shitposting_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET shitposting_channel_id = excluded.shitposting_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_instagram_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, instagram_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET instagram_channel_id = excluded.instagram_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_news_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, news_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET news_channel_id = excluded.news_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_free_games_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, free_games_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET free_games_channel_id = excluded.free_games_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_nsfw_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, nsfw_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET nsfw_channel_id = excluded.nsfw_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_rule34_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, rule34_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET rule34_channel_id = excluded.rule34_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_porn_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, porn_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET porn_channel_id = excluded.porn_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_hentai_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, hentai_channel_id) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET hentai_channel_id = excluded.hentai_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_auto_react_enabled(db: &SqlitePool, guild_id: &str, enabled: bool) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, auto_react_enabled) VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET auto_react_enabled = excluded.auto_react_enabled",
    )
    .bind(guild_id)
    .bind(if enabled { 1 } else { 0 })
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_posting_interval(db: &SqlitePool, guild_id: &str, secs: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, posting_interval_secs) \
         VALUES (?, ?) \
         ON CONFLICT(guild_id) DO UPDATE SET posting_interval_secs = excluded.posting_interval_secs",
    )
    .bind(guild_id)
    .bind(secs)
    .execute(db)
    .await?;
    Ok(())
}

/// Return all guild configs.
pub async fn get_all_guild_configs(db: &SqlitePool) -> Result<Vec<GuildConfig>> {
    let rows = sqlx::query(
        "SELECT guild_id, meme_channel_id, posting_interval_secs, \
                brainrot_channel_id, shitposting_channel_id, instagram_channel_id, \
                news_channel_id, free_games_channel_id, nsfw_channel_id, rule34_channel_id, \
                porn_channel_id, hentai_channel_id, auto_react_enabled \
         FROM guild_config",
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| GuildConfig {
            guild_id: r.get("guild_id"),
            meme_channel_id: r.get("meme_channel_id"),
            posting_interval_secs: r.get("posting_interval_secs"),
            brainrot_channel_id: r.get("brainrot_channel_id"),
            shitposting_channel_id: r.get("shitposting_channel_id"),
            instagram_channel_id: r.get("instagram_channel_id"),
            news_channel_id: r.get("news_channel_id"),
            free_games_channel_id: r.get("free_games_channel_id"),
            nsfw_channel_id: r.get("nsfw_channel_id"),
            rule34_channel_id: r.get("rule34_channel_id"),
            porn_channel_id: r.get("porn_channel_id"),
            hentai_channel_id: r.get("hentai_channel_id"),
            auto_react_enabled: r.get::<i64, _>("auto_react_enabled") != 0,
        })
        .collect())
}

// ────────────────────────────────────────────────────────────────────────────
// Reaction channels
// ────────────────────────────────────────────────────────────────────────────

pub async fn add_reaction_channel(
    db: &SqlitePool,
    guild_id: &str,
    channel_id: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO reaction_channels (guild_id, channel_id) VALUES (?, ?)",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn remove_reaction_channel(
    db: &SqlitePool,
    guild_id: &str,
    channel_id: &str,
) -> Result<()> {
    sqlx::query("DELETE FROM reaction_channels WHERE guild_id = ? AND channel_id = ?")
        .bind(guild_id)
        .bind(channel_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_reaction_channels(db: &SqlitePool, guild_id: &str) -> Result<Vec<String>> {
    let rows =
        sqlx::query("SELECT channel_id FROM reaction_channels WHERE guild_id = ?")
            .bind(guild_id)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|r| r.get("channel_id")).collect())
}

pub async fn is_reaction_channel(
    db: &SqlitePool,
    guild_id: &str,
    channel_id: &str,
) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM reaction_channels WHERE guild_id = ? AND channel_id = ? LIMIT 1",
    )
    .bind(guild_id)
    .bind(channel_id)
    .fetch_optional(db)
    .await?;
    Ok(row.is_some())
}

// ────────────────────────────────────────────────────────────────────────────
// Reaction users
// ────────────────────────────────────────────────────────────────────────────

pub async fn add_reaction_user(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO reaction_users (guild_id, user_id) VALUES (?, ?)")
        .bind(guild_id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_reaction_user(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM reaction_users WHERE guild_id = ? AND user_id = ?")
        .bind(guild_id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_reaction_users(db: &SqlitePool, guild_id: &str) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT user_id FROM reaction_users WHERE guild_id = ?")
        .bind(guild_id)
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("user_id")).collect())
}

pub async fn is_reaction_user(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<bool> {
    let row =
        sqlx::query("SELECT 1 FROM reaction_users WHERE guild_id = ? AND user_id = ? LIMIT 1")
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;
    Ok(row.is_some())
}

// ────────────────────────────────────────────────────────────────────────────
// Channel and User Custom Emojis (Overrides)
// ────────────────────────────────────────────────────────────────────────────

pub async fn add_channel_emoji(db: &SqlitePool, guild_id: &str, channel_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO channel_emojis (guild_id, channel_id, emoji) VALUES (?, ?, ?)")
        .bind(guild_id)
        .bind(channel_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_channel_emoji(db: &SqlitePool, guild_id: &str, channel_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("DELETE FROM channel_emojis WHERE guild_id = ? AND channel_id = ? AND emoji = ?")
        .bind(guild_id)
        .bind(channel_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_channel_emojis(db: &SqlitePool, guild_id: &str, channel_id: &str) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT emoji FROM channel_emojis WHERE guild_id = ? AND channel_id = ?")
        .bind(guild_id)
        .bind(channel_id)
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("emoji")).collect())
}

pub async fn add_user_emoji(db: &SqlitePool, guild_id: &str, user_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO user_emojis (guild_id, user_id, emoji) VALUES (?, ?, ?)")
        .bind(guild_id)
        .bind(user_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_user_emoji(db: &SqlitePool, guild_id: &str, user_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("DELETE FROM user_emojis WHERE guild_id = ? AND user_id = ? AND emoji = ?")
        .bind(guild_id)
        .bind(user_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_user_emojis(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT emoji FROM user_emojis WHERE guild_id = ? AND user_id = ?")
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("emoji")).collect())
}

// ────────────────────────────────────────────────────────────────────────────
// Reaction emojis (default fallback list)
// ────────────────────────────────────────────────────────────────────────────

pub async fn add_emoji(db: &SqlitePool, guild_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO reaction_emojis (guild_id, emoji) VALUES (?, ?)")
        .bind(guild_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_emoji(db: &SqlitePool, guild_id: &str, emoji: &str) -> Result<()> {
    sqlx::query("DELETE FROM reaction_emojis WHERE guild_id = ? AND emoji = ?")
        .bind(guild_id)
        .bind(emoji)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_emojis(db: &SqlitePool, guild_id: &str) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT emoji FROM reaction_emojis WHERE guild_id = ?")
        .bind(guild_id)
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("emoji")).collect())
}

// ────────────────────────────────────────────────────────────────────────────
// Seen posts (deduplication)
// ────────────────────────────────────────────────────────────────────────────

pub async fn is_post_seen(db: &SqlitePool, guild_id: &str, post_id: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM seen_posts WHERE guild_id = ? AND post_id = ? LIMIT 1",
    )
    .bind(guild_id)
    .bind(post_id)
    .fetch_optional(db)
    .await?;
    Ok(row.is_some())
}

pub async fn mark_post_seen(db: &SqlitePool, guild_id: &str, post_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO seen_posts (guild_id, post_id) VALUES (?, ?)",
    )
    .bind(guild_id)
    .bind(post_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn prune_old_seen_posts(db: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM seen_posts WHERE posted_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", days))
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

// ────────────────────────────────────────────────────────────────────────────
// Seen news
// ────────────────────────────────────────────────────────────────────────────

pub async fn is_news_seen(db: &SqlitePool, guild_id: &str, article_id: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM seen_news WHERE guild_id = ? AND article_id = ? LIMIT 1",
    )
    .bind(guild_id)
    .bind(article_id)
    .fetch_optional(db)
    .await?;
    Ok(row.is_some())
}

pub async fn mark_news_seen(db: &SqlitePool, guild_id: &str, article_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO seen_news (guild_id, article_id) VALUES (?, ?)",
    )
    .bind(guild_id)
    .bind(article_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn prune_old_seen_news(db: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM seen_news WHERE seen_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", days))
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

// ────────────────────────────────────────────────────────────────────────────
// Seen giveaways
// ────────────────────────────────────────────────────────────────────────────

pub async fn is_giveaway_seen(db: &SqlitePool, guild_id: &str, giveaway_id: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM seen_giveaways WHERE guild_id = ? AND giveaway_id = ? LIMIT 1",
    )
    .bind(guild_id)
    .bind(giveaway_id)
    .fetch_optional(db)
    .await?;
    Ok(row.is_some())
}

pub async fn mark_giveaway_seen(db: &SqlitePool, guild_id: &str, giveaway_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO seen_giveaways (guild_id, giveaway_id) VALUES (?, ?)",
    )
    .bind(guild_id)
    .bind(giveaway_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn prune_old_seen_giveaways(db: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM seen_giveaways WHERE seen_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", days))
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}
