use feed_rs::parser;
use tracing::{debug, warn};

use super::models::CocUpdate;

// ── RSS / Atom feeds ─────────────────────────────────────────────────────────
// Primary:  r/ClashOfClans official posts (Reddit RSS, no auth needed)
// Secondary: Supercell blog via unofficial Supercell inbox RSS
// Tertiary:  Clash of Clans YouTube (for update trailer videos)

const REDDIT_COC_RSS: &str =
    "https://www.reddit.com/r/ClashOfClans/search.rss?q=flair%3A%22Official%22&sort=new&restrict_sr=1";

const REDDIT_COC_NEW_RSS: &str =
    "https://www.reddit.com/r/ClashOfClans/new/.rss";

const COC_YOUTUBE_RSS: &str =
    "https://www.youtube.com/feeds/videos.xml?channel_id=UCjRfAVJHGMpFdNqvJA-yfFg";

// ── Keyword detection ─────────────────────────────────────────────────────────

/// Returns the tag for a CoC post based on its title, or None if we should skip it.
fn classify(title: &str) -> Option<&'static str> {
    let t = title.to_lowercase();

    // Free reward / magic item / gift / code
    if t.contains("free") || t.contains("reward") || t.contains("gift")
        || t.contains("magic item") || t.contains("code") || t.contains("redeem")
        || t.contains("giveaway") || t.contains("gem")
    {
        return Some("🎁 Free Reward");
    }
    // Game updates / patch notes / maintenance
    if t.contains("update") || t.contains("patch") || t.contains("season")
        || t.contains("sneak peek") || t.contains("balance") || t.contains("maintenance")
        || t.contains("new hero") || t.contains("new troop") || t.contains("new spell")
        || t.contains("town hall") || t.contains("builder hall")
    {
        return Some("⚔️ Update");
    }
    // Clan war / cwl
    if t.contains("cwl") || t.contains("clan war") || t.contains("war league") {
        return Some("🏆 Clan War League");
    }
    // Events / challenges
    if t.contains("event") || t.contains("challenge") || t.contains("championship")
        || t.contains("esport") || t.contains("esports") || t.contains("qualifier")
        || t.contains("legend league") || t.contains("gold pass")
    {
        return Some("🏅 Event");
    }
    // Official YouTube
    if t.contains("official") || t.contains("trailer") || t.contains("developer")
        || t.contains("dev") || t.contains("announcement") || t.contains("reveal")
    {
        return Some("📢 Announcement");
    }

    None // skip non-CoC posts
}

// ── Simple HTML stripper ──────────────────────────────────────────────────────

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; out.push(' '); }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;",  "&")
       .replace("&lt;",   "<")
       .replace("&gt;",   ">")
       .replace("&quot;", "\"")
       .replace("&#39;",  "'")
       .replace("&nbsp;", " ")
       .split_whitespace()
       .collect::<Vec<_>>()
       .join(" ")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s.char_indices().map(|(i,_)| i).take(max-1).last().unwrap_or(max-1);
        format!("{}…", &s[..end])
    }
}

// ── Fetch helpers ─────────────────────────────────────────────────────────────

async fn fetch_rss(
    client: &reqwest::Client,
    url: &str,
    source: &str,
    is_youtube: bool,
) -> Vec<CocUpdate> {
    let result: anyhow::Result<Vec<CocUpdate>> = async {
        let bytes = client
            .get(url)
            .header("User-Agent", "discord-bot/1.0 (CoC watcher)")
            .header("Accept", "application/rss+xml, application/atom+xml, */*")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let feed = parser::parse(bytes.as_ref())
            .map_err(|e| anyhow::anyhow!("Parse error for {url}: {e}"))?;

        let items = feed.entries.into_iter().take(15).filter_map(|entry| {
            let title     = entry.title.as_ref()?.content.clone();
            let link      = entry.links.first()?.href.clone();
            let summary   = entry.summary.map(|t| t.content);
            let content   = entry.content.and_then(|c| c.body);
            let published = entry.published;
            let media     = entry.media;

            // For Reddit RSS: the raw title can be noisy — filter by keyword
            // For YouTube: always include (CoC official channel is already filtered)
            let tag = if is_youtube {
                "📺 Update Video"
            } else {
                classify(&title)?
            };

            let description = summary
                .or(content)
                .map(|html| truncate(&strip_html(&html), 220));

            let image_url = media.iter().find_map(|m| {
                m.content.iter().find_map(|c| c.url.as_ref().map(|u| u.as_str().to_string()))
            });

            let id = format!("coc::{}::{}", source, link);

            Some(CocUpdate {
                id,
                title,
                description,
                url: link,
                image_url,
                source: source.to_string(),
                published_at: published,
                tag: tag.to_string(),
            })
        }).collect();

        Ok(items)
    }.await;

    match result {
        Ok(items) => { debug!("CoC {source}: {} items", items.len()); items }
        Err(e)    => { warn!("CoC {source} RSS failed: {:#}", e); vec![] }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub async fn fetch_all_updates(client: &reqwest::Client) -> Vec<CocUpdate> {
    // Fetch all sources concurrently
    let (mut reddit_official, mut reddit_new, mut youtube) = tokio::join!(
        fetch_rss(client, REDDIT_COC_RSS,     "r/ClashOfClans Official", false),
        fetch_rss(client, REDDIT_COC_NEW_RSS, "r/ClashOfClans",          false),
        fetch_rss(client, COC_YOUTUBE_RSS,    "CoC YouTube",              true),
    );

    let mut all: Vec<CocUpdate> = Vec::new();
    all.append(&mut reddit_official);
    all.append(&mut reddit_new);
    all.append(&mut youtube);

    // Deduplicate by id
    let mut seen = std::collections::HashSet::new();
    all.retain(|u| seen.insert(u.id.clone()));

    debug!("CoC fetcher: {} total updates found", all.len());
    all
}
