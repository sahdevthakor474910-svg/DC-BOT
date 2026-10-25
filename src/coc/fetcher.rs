use tracing::info;

use super::models::CocUpdate;

// Only show posts from the last N hours — prevents old content flooding on first run
const MAX_AGE_HOURS: i64 = 48;

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&amp;",  "&")
        .replace("&lt;",   "<")
        .replace("&gt;",   ">")
        .replace("&quot;", "\"")
        .replace("&#39;",  "'")
        .replace("&nbsp;", " ");

    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct CocTweet {
    id: String,
    text: String,
    link: String,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn fetch_coc_tweets(http: &reqwest::Client, limit: usize) -> Vec<CocTweet> {
    let instances = &[
        "https://nitter.net",
        "https://nitter.privacyredirect.com",
        "https://nitter.poast.org",
    ];

    for &base_url in instances {
        let url = format!("{}/ClashofClans/rss", base_url);
        let res = match http.get(&url).timeout(std::time::Duration::from_secs(8)).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let bytes = match res.bytes().await {
            Ok(b) => b,
            _ => continue,
        };

        let feed = match feed_rs::parser::parse(bytes.as_ref()) {
            Ok(f) => f,
            _ => continue,
        };

        let mut tweets = Vec::new();
        for entry in feed.entries {
            let id = entry.id.clone();
            let doc_link = entry.links.first().map(|l| l.href.clone()).unwrap_or_else(|| id.clone());
            let tweet_id = if id.chars().all(|c| c.is_ascii_digit()) {
                id
            } else {
                let parsed = doc_link.split("/status/").nth(1)
                    .and_then(|s| s.split('#').next())
                    .and_then(|s| s.split('?').next())
                    .unwrap_or("")
                    .to_string();
                if parsed.is_empty() {
                    continue;
                }
                parsed
            };

            let title = entry.title.map(|t| t.content).unwrap_or_default();
            let summary_html = entry.summary.map(|s| s.content).unwrap_or_default();
            let text = if !summary_html.is_empty() {
                strip_html(&summary_html)
            } else {
                title
            };

            let link = format!("https://twitter.com/ClashofClans/status/{}", tweet_id);

            tweets.push(CocTweet {
                id: tweet_id,
                text,
                link,
                published_at: entry.published,
            });

            if tweets.len() >= limit {
                break;
            }
        }

        if !tweets.is_empty() {
            return tweets;
        }
    }

    vec![]
}

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

pub async fn fetch_all_updates(client: &reqwest::Client) -> Vec<CocUpdate> {
    let tweets = fetch_coc_tweets(client, 10).await;

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
