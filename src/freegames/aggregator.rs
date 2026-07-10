use std::collections::HashSet;

use tracing::debug;

use super::epic;
use super::models::FreeGame;

/// Fetch from all configured sources and merge into one deduplicated list.
pub async fn fetch_all(client: &reqwest::Client) -> Vec<FreeGame> {
    let mut games: Vec<FreeGame> = Vec::new();

    // ── Epic Games ────────────────────────────────────────────────────────
    let mut epic = epic::fetch_free_games(client).await;
    debug!("Epic: {} free game(s) found", epic.len());
    games.append(&mut epic);

    // Deduplicate by ID (in case the same game comes from multiple calls)
    let mut seen_ids = std::collections::HashSet::new();
    games.retain(|g| seen_ids.insert(g.id.clone()));

    games
}
