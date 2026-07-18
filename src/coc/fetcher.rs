use tracing::{info, warn};

use crate::twitter::client::TwitterClient;
use super::models::CocUpdate;

// Only show posts from the last N hours — prevents old content flooding on first run
const MAX_AGE_HOURS: i64 = 48;

// =============================================================================
// Keyword classifier
// =============================================================================

/// Maps a post text to an emoji tag. Falls back to a generic announcement tag.
fn classify(text: &str) -> &'static str {
    let t = text.to_lowercase();

    // Free reward / gifts / store freebies
    if t.contains("free") || t.contains("reward") || t.contains("gift")
        || t.contains("magic item") || t.contains("code") || t.contains("redeem")
        || t.contains("giveaway") || t.contains("gem") || t.contains("free gift")
        || t.contains("special offer") || t.contains("store")
    {
        return "🎁 Free Reward";
    }

    // Season updates, Sneak Peeks, Maintenance, balance changes, Town Hall updates
    if t.contains("update") || t.contains("patch") || t.contains("season")
        || t.contains("sneak peek") || t.contains("balance") || t.contains("maintenance")
        || t.contains("new hero") || t.contains("new troop") || t.contains("new spell")
        || t.contains("town hall") || t.contains("builder hall") || t.contains("changes")
        || t.contains("teaser") || t.contains("coming soon") || t.contains("new feature")
        || t.contains("trailer") || t.contains("reveal") || t.contains("dev update")
        || t.contains("developer update") || t.contains("sneak peak")
    {
        return "⚔️ Update";
    }

    // Clan Games, Gold Pass, Events, Challenges (e.g. spotlight etc)
    if t.contains("event") || t.contains("challenge") || t.contains("championship")
        || t.contains("esport") || t.contains("qualifier") || t.contains("legend league")
        || t.contains("gold pass") || t.contains("spotlight") || t.contains("calendar")
        || t.contains("clan games") || t.contains("war league") || t.contains("cwl")
    {
        return "🏅 Event";
    }

    "📢 Announcement"
}

// =============================================================================
// Helpers
// =============================================================================

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s.char_indices().map(|(i, _)| i).take(max - 1).last().unwrap_or(max - 1);
        format!("{}…", &s[..end])
    }
}

/// True if the post is within MAX_AGE_HOURS of now.
fn is_recent(ts: Option<chrono::DateTime<chrono::Utc>>) -> bool {
    match ts {
        Some(t) => (chrono::Utc::now() - t).num_hours() <= MAX_AGE_HOURS,
        None    => false,
    }
}

// =============================================================================
// Public API
// =============================================================================

pub async fn fetch_all_updates(_client: &reqwest::Client) -> Vec<CocUpdate> {
    let twitter_client = match TwitterClient::new() {
        Ok(tc) => tc,
        Err(e) => {
            warn!("CoC fetcher: failed to initialize TwitterClient: {}", e);
            return vec![];
        }
    };

    let tweets = match twitter_client.fetch_tweets("ClashofClans", 10).await {
        Ok(t) => t,
        Err(e) => {
            warn!("CoC fetcher: failed to fetch tweets: {}", e);
            vec![]
        }
    };

    let mut updates = Vec::new();
    for tweet in tweets {
        // Enforce 48-hour recency gate on tweets
        if !is_recent(tweet.published_at) {
            continue;
        }

        let tag = classify(&tweet.text);

        let text_lines: Vec<&str> = tweet.text.split('\n').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let first_line = text_lines.first().cloned().unwrap_or("");
        let title = if first_line.is_empty() {
            truncate(&tweet.text, 80)
        } else if first_line.chars().count() > 80 {
            truncate(first_line, 80)
        } else {
            first_line.to_string()
        };

        // Always include the full tweet text as description (shows the full body in the embed).
        let description = Some(truncate(&tweet.text, 900));

        updates.push(CocUpdate {
            id: format!("coc::x::{}", tweet.id),
            title,
            description,
            url: tweet.link.clone(),
            image_url: None, // simple, image in text link is followed by Discord auto-preview anyway
            source: "𝕏 @ClashofClans".to_string(),
            published_at: tweet.published_at,
            tag: tag.to_string(),
        });
    }

    info!("CoC X: {} item(s) fetched", updates.len());
    updates
}
