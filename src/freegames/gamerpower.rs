use anyhow::Result;
use serde::Deserialize;
use tracing::{debug, warn};

use super::models::FreeGame;

// ── GamerPower public API ────────────────────────────────────────────────────
// No API key required. Returns all active PC game giveaways across:
//   Epic Games, Steam, Itch.io, IndieGala, GOG, Stove, DRM-Free, etc.
//
const GAMERPOWER_URL: &str =
    "https://www.gamerpower.com/api/giveaways?platform=pc&type=game&sort-by=date";

// ── Serde models ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GamerPowerGame {
    id: u64,
    title: String,
    worth: Option<String>,
    thumbnail: Option<String>,
    description: Option<String>,
    open_giveaway_url: String,
    platforms: String,
    end_date: Option<String>,
    status: String,
}

// ── Public fetcher ───────────────────────────────────────────────────────────

pub async fn fetch_free_games(client: &reqwest::Client) -> Vec<FreeGame> {
    match try_fetch(client).await {
        Ok(games) => games,
        Err(e) => {
            warn!("GamerPower API error: {:#}", e);
            vec![]
        }
    }
}

async fn try_fetch(client: &reqwest::Client) -> Result<Vec<FreeGame>> {
    let games: Vec<GamerPowerGame> = client
        .get(GAMERPOWER_URL)
        .header("User-Agent", "discord-bot/1.0")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut results = Vec::new();

    for game in games {
        // Only include active giveaways
        if game.status.to_lowercase() != "active" {
            continue;
        }

        // Parse end date — "N/A" means no known expiry (keep it)
        let end_date = game.end_date.as_deref().and_then(|s| {
            if s == "N/A" || s.trim().is_empty() {
                return None;
            }
            // Format: "2026-07-16 23:59:00"
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|ndt| ndt.and_utc())
        });

        // Original price — skip if $0.00 / Free
        let original_price = game.worth.as_deref().and_then(|w| {
            if w == "$0.00" || w.to_lowercase() == "free" || w == "N/A" {
                None
            } else {
                Some(w.to_string())
            }
        });

        // Derive a clean store name from the `platforms` field
        let store = derive_store(&game.platforms);

        // Claim instructions hint
        let claim_instructions = format!(
            "Visit the link and follow the instructions to claim this free game from {}!",
            store
        );

        debug!("Found free game via GamerPower: {} ({})", game.title, store);

        results.push(FreeGame {
            id: format!("gamerpower::{}", game.id),
            title: game.title,
            description: game.description,
            original_price,
            store,
            url: game.open_giveaway_url,
            thumbnail_url: game.thumbnail,
            end_date,
            claim_instructions,
        });
    }

    Ok(results)
}

/// Pick the most recognisable store name from the comma-separated platforms string.
fn derive_store(platforms: &str) -> String {
    let p = platforms.to_lowercase();
    if p.contains("epic") {
        "Epic Games".to_string()
    } else if p.contains("steam") {
        "Steam".to_string()
    } else if p.contains("gog") {
        "GOG".to_string()
    } else if p.contains("itch") {
        "Itch.io".to_string()
    } else if p.contains("indiegala") {
        "IndieGala".to_string()
    } else if p.contains("ubisoft") {
        "Ubisoft".to_string()
    } else if p.contains("stove") {
        "Stove".to_string()
    } else if p.contains("drm-free") {
        "DRM-Free".to_string()
    } else {
        "PC".to_string()
    }
}
