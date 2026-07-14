use anyhow::Result;
use sqlx::SqlitePool;
use tracing::info;

/// Run all migration SQL files on first boot.
pub async fn run_migrations(db: &SqlitePool) -> Result<()> {
    info!("Running database migrations…");

    // Enable WAL mode for better concurrent read performance
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(db)
        .await?;

    // ── Migration 001: initial schema ─────────────────────────────────────────
    let m1 = include_str!("../../migrations/001_initial.sql");
    run_statements(db, m1).await?;

    // ── Migration 002: v2 upgrade ─────────────────────────────────────────────
    // Each ALTER TABLE is run individually. Duplicate-column errors are silently
    // swallowed so re-running the bot on an already-migrated DB is safe.
    let m2 = include_str!("../../migrations/002_upgrade.sql");
    run_migration_alter(db, m2, "002").await?;

    // ── Migration 003: NSFW + rule34 / porn / hentai channels ─────────────────
    let m3 = include_str!("../../migrations/003_nsfw_and_rule34.sql");
    run_migration_alter(db, m3, "003").await?;

    // ── Migration 004: JAV channel + seen_jav table ───────────────────────────
    let m4 = include_str!("../../migrations/004_jav_channel.sql");
    run_migration_alter(db, m4, "004").await?;

    // ── Migration 005: Porn video channel (RedTube) + seen_porn_videos table ─────────
    let m5 = include_str!("../../migrations/005_porn_videos.sql");
    run_migration_alter(db, m5, "005").await?;

    // ── Migration 006: ok.xxx channel + seen_okxxx table ──────────────────────
    let m6 = include_str!("../../migrations/006_okxxx_channel.sql");
    run_migration_alter(db, m6, "006").await?;

    // ── Migration 007: per-guild user blocklist ────────────────────────────────
    let m7 = include_str!("../../migrations/007_blocked_users.sql");
    run_statements(db, m7).await?;

    // ── Migration 008: Clash of Clans channel + seen_coc dedup table ──────────
    let m8 = include_str!("../../migrations/008_coc_channel.sql");
    run_migration_alter(db, m8, "008").await?;

    // ── Migration 009: X / Twitter channel + seen_tweets dedup table ──────────
    let m9 = include_str!("../../migrations/009_twitter_channel.sql");
    run_migration_alter(db, m9, "009").await?;

    info!("Database migrations complete");
    Ok(())
}

/// Execute each semicolon-delimited statement from the initial migration.
/// Lines starting with `--` are stripped first to avoid empty/comment statements.
async fn run_statements(db: &SqlitePool, sql: &str) -> Result<()> {
    for stmt in sql.split(';') {
        let stmt = stmt
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(db).await?;
        }
    }
    Ok(())
}

/// Execute an ALTER-TABLE-heavy migration, tolerating "duplicate column" errors.
/// Safe to re-run on an already-migrated DB.
async fn run_migration_alter(db: &SqlitePool, sql: &str, label: &str) -> Result<()> {
    for stmt in sql.split(';') {
        // Strip comment lines
        let stmt = stmt
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let stmt = stmt.trim().to_string();
        if stmt.is_empty() {
            continue;
        }

        if let Err(e) = sqlx::query(&stmt).execute(db).await {
            let msg = e.to_string();
            let is_alter = stmt.to_uppercase().starts_with("ALTER TABLE");
            let is_dup = msg.contains("duplicate column name") || msg.contains("already exists");
            if is_alter && is_dup {
                // Safe to ignore — column already exists from a previous run
                continue;
            }
            return Err(anyhow::anyhow!("Migration {} failed on:\n{}\nError: {}", label, stmt, e));
        }
    }
    Ok(())
}
