import re

file_path = 'c:/Users/DELL/OneDrive/Pictures/dc bot/src/db/queries.rs'
with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. struct GuildConfig
content = content.replace(
    'pub hanime_channel_id: Option<String>,        // added via migration 014 (Hanime)',
    'pub hanime_channel_id: Option<String>,        // added via migration 014 (Hanime)\n    pub pornclips_channel_id: Option<String>,     // added via migration 015 (Porn Clips)'
)

# 2. SELECT queries
content = content.replace(
    'javhd_channel_id, hanime_channel_id, auto_react_enabled',
    'javhd_channel_id, hanime_channel_id, pornclips_channel_id, auto_react_enabled'
)

# 3. get_or_create_guild & get_all_guild_configs instantiation
content = content.replace(
    'hanime_channel_id: row.get("hanime_channel_id"),',
    'hanime_channel_id: row.get("hanime_channel_id"),\n        pornclips_channel_id: row.get("pornclips_channel_id"),'
)

# 4. Add pornclips functions (set_pornclips_channel, is_pornclips_seen, mark_pornclips_seen, prune_old_seen_pornclips)
func_str = '''
pub async fn set_pornclips_channel(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, pornclips_channel_id) VALUES (?, ?) \\
         ON CONFLICT(guild_id) DO UPDATE SET pornclips_channel_id = excluded.pornclips_channel_id",
    )
    .bind(guild_id)
    .bind(channel_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn is_pornclips_seen(db: &SqlitePool, guild_id: &str, item_id: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM seen_pornclips WHERE guild_id = ? AND item_id = ? LIMIT 1",
    )
    .bind(guild_id)
    .bind(item_id)
    .fetch_optional(db)
    .await?;
    Ok(row.is_some())
}

pub async fn mark_pornclips_seen(db: &SqlitePool, guild_id: &str, item_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO seen_pornclips (guild_id, item_id) VALUES (?, ?)",
    )
    .bind(guild_id)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn prune_old_seen_pornclips(db: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM seen_pornclips WHERE seen_at < strftime('%s','now') - ? * 86400",
    )
    .bind(days)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}
'''

content += func_str

# Check and update backup_data function 
content = content.replace(
    '"hanime_channel_id": cfg.hanime_channel_id.clone(),',
    '"hanime_channel_id": cfg.hanime_channel_id.clone(),\n            "pornclips_channel_id": cfg.pornclips_channel_id.clone(),'
)

content = content.replace(
    '"hanime_channel_id": cfg.hanime_channel_id,',
    '"hanime_channel_id": cfg.hanime_channel_id,\n            "pornclips_channel_id": cfg.pornclips_channel_id,'
)

content = content.replace(
    'hanime_channel_id = ?,',
    'hanime_channel_id = ?,\n                   pornclips_channel_id = ?,'
)

# restore_data
content = content.replace(
    '.bind(cfg.get("hanime_channel_id").and_then(|v| v.as_str()))',
    '.bind(cfg.get("hanime_channel_id").and_then(|v| v.as_str()))\n                .bind(cfg.get("pornclips_channel_id").and_then(|v| v.as_str()))'
)

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated queries.rs')
