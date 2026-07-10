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
    run_migration_002(db, m2).await?;

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

/// Execute migration 002, tolerating "duplicate column" errors for ALTER TABLE.
async fn run_migration_002(db: &SqlitePool, sql: &str) -> Result<()> {
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
            return Err(anyhow::anyhow!("Migration 002 failed on:\n{}\nError: {}", stmt, e));
        }
    }
    Ok(())
}
