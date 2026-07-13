use tracing::{debug, info};

use super::epic;
use super::gamerpower;
use super::models::FreeGame;

/// Fetch from all configured sources and merge into one deduplicated list.
///
/// Primary source:  GamerPower API — aggregates Epic, Steam, Itch.io,
///                  IndieGala, GOG, Stove, DRM-Free and more.
/// Fallback source: Epic Games direct API — used if GamerPower returns no
///                  Epic games (e.g. GamerPower is down or rate-limited).
pub async fn fetch_all(client: &reqwest::Client) -> Vec<FreeGame> {
    let mut games: Vec<FreeGame> = Vec::new();

    // ── GamerPower (primary) ──────────────────────────────────────────────
    let mut gp = gamerpower::fetch_free_games(client).await;
    debug!("GamerPower: {} free game(s) found", gp.len());

    // Count how many Epic results GamerPower gave us
    let gp_has_epic = gp.iter().any(|g| g.store == "Epic Games");
    games.append(&mut gp);

    // ── Epic direct API (fallback, only if GamerPower missed Epic) ────────
    if !gp_has_epic {
        info!("GamerPower returned no Epic Games entries — falling back to direct Epic API");
        let mut epic = epic::fetch_free_games(client).await;
        debug!("Epic direct API: {} free game(s) found", epic.len());
        games.append(&mut epic);
    }

    // ── Deduplicate by ID ─────────────────────────────────────────────────
    let mut seen_ids = std::collections::HashSet::new();
    games.retain(|g| seen_ids.insert(g.id.clone()));

    info!("Free games aggregator: {} total unique giveaway(s)", games.len());
    games
}

