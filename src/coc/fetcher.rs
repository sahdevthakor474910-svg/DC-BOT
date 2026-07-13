use feed_rs::parser;
use tracing::{debug, info, warn};

use super::models::CocUpdate;

// =============================================================================
// Official CoC news sources
//
// 1. Supercell Inbox RSS — 3 feeds (news / events / community)
//    Project: https://github.com/skrwo/supercell-inbox-rss (GitHub Pages)
//    Scrapes Supercell's official in-game CMS every hour at :25.
//    Identical to what you see in the "News" tab inside the game.
//
// 2. Official CoC YouTube channel
//    Channel ID verified via "Copy channel ID" on @ClashofClans YouTube page.
//    Trailers, developer updates, season reveals.
//
// 3. r/ClashOfClans — Official flair only
//    Supercell community managers (u/ClashOfClans) post announcements here.
// =============================================================================

// Supercell Inbox RSS feeds (raw.githubusercontent.com paths)
const SUPERCELL_NEWS_RSS: &str =
    "https://raw.githubusercontent.com/skrwo/supercell-inbox-rss/main/rss/clashofclans/en/news.xml";

const SUPERCELL_EVENTS_RSS: &str =
    "https://raw.githubusercontent.com/skrwo/supercell-inbox-rss/main/rss/clashofclans/en/events.xml";

const SUPERCELL_COMMUNITY_RSS: &str =
    "https://raw.githubusercontent.com/skrwo/supercell-inbox-rss/main/rss/clashofclans/en/community.xml";


// YouTube channel ID verified from https://www.youtube.com/@ClashofClans
const COC_YOUTUBE_RSS: &str =
    "https://www.youtube.com/feeds/videos.xml?channel_id=UCD1Em4q90ZUK2R5HKesszJg";

// Reddit Official-flair posts, sorted by new, last week only
const REDDIT_OFFICIAL_RSS: &str =
    "https://www.reddit.com/r/ClashOfClans/search.rss?q=flair%3A%22Official%22&sort=new&restrict_sr=1&t=week";

// Only show posts from the last N hours — prevents old content flooding on first run
const MAX_AGE_HOURS: i64 = 48;

// =============================================================================
// Keyword classifier
// =============================================================================

/// Maps a post title to an emoji tag.  Returns None to drop unrelated posts.
fn classify(title: &str) -> Option<&'static str> {
    let t = title.to_lowercase();

    if t.contains("free") || t.contains("reward") || t.contains("gift")
        || t.contains("magic item") || t.contains("code") || t.contains("redeem")
        || t.contains("giveaway") || t.contains("gem") || t.contains("free gift")
        || t.contains("clan games") || t.contains("magic items")
    {
        return Some("🎁 Free Reward");
    }
    if t.contains("cwl") || t.contains("clan war league") || t.contains("war league") {
        return Some("🏆 Clan War League");
    }
    if t.contains("event") || t.contains("challenge") || t.contains("championship")
        || t.contains("esport") || t.contains("qualifier") || t.contains("legend league")
        || t.contains("gold pass") || t.contains("spotlight") || t.contains("calendar")
    {
        return Some("🏅 Event");
    }
    if t.contains("update") || t.contains("patch") || t.contains("season")
        || t.contains("sneak peek") || t.contains("balance") || t.contains("maintenance")
        || t.contains("new hero") || t.contains("new troop") || t.contains("new spell")
        || t.contains("town hall") || t.contains("builder hall") || t.contains("changes")
        || t.contains("teaser") || t.contains("coming soon") || t.contains("new feature")
    {
        return Some("⚔️ Update");
    }
    if t.contains("announcement") || t.contains("reveal") || t.contains("official")
        || t.contains("developer") || t.contains("community")
    {
        return Some("📢 Announcement");
    }

    Some("📰 News")
}

// =============================================================================
// Helpers
// =============================================================================

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
// RSS fetcher
// =============================================================================

/// Fetch one RSS/Atom feed and map entries to CocUpdate.
///
/// * `is_youtube`         — skip keyword filter, always tag as "📺 Update Video"
/// * `is_supercell_inbox` — ALL items are official; bypass date gate & accept all keywords
async fn fetch_rss(
    client: &reqwest::Client,
    url: &str,
    source: &str,
    is_youtube: bool,
    is_supercell_inbox: bool,
) -> Vec<CocUpdate> {
    let result: anyhow::Result<Vec<CocUpdate>> = async {
        let bytes = client
            .get(url)
            .header("User-Agent", "discord-coc-bot/2.0 (contact: github.com/dc-bot)")
            .header("Accept", "application/rss+xml, application/atom+xml, */*")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let feed = parser::parse(bytes.as_ref())
            .map_err(|e| anyhow::anyhow!("Parse error for {url}: {e}"))?;

        let items = feed.entries.into_iter().take(20).filter_map(|entry| {
            let title     = entry.title.as_ref()?.content.clone();
            let link      = entry.links.first()?.href.clone();
            let summary   = entry.summary.map(|t| t.content);
            let content   = entry.content.and_then(|c| c.body);
            let published = entry.published;
            let media     = entry.media;

            // For Supercell Inbox: accept all (it's the official CMS, always relevant)
            // For others: enforce 48-hour recency gate
            if !is_supercell_inbox && !is_youtube && !is_recent(published) {
                return None;
            }

            let tag: &str = if is_youtube {
                "📺 Update Video"
            } else if is_supercell_inbox {
                classify(&title).unwrap_or("📰 News")
            } else {
                classify(&title)?
            };

            let description = summary
                .or(content)
                .map(|html| truncate(&strip_html(&html), 300));

            let image_url = media.iter().find_map(|m| {
                m.content.iter().find_map(|c| c.url.as_ref().map(|u| u.as_str().to_string()))
            });

            Some(CocUpdate {
                id:           format!("coc::{}::{}", source, link),
                title,
                description,
                url:          link,
                image_url,
                source:       source.to_string(),
                published_at: published,
                tag:          tag.to_string(),
            })
        }).collect();

        Ok(items)
    }.await;

    match result {
        Ok(items) => {
            if !items.is_empty() {
                info!("CoC {source}: {} item(s) fetched", items.len());
            } else {
                debug!("CoC {source}: 0 new items this cycle");
            }
            items
        }
        Err(e) => {
            warn!("CoC {source} unavailable: {:#}", e);
            vec![]
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

pub async fn fetch_all_updates(client: &reqwest::Client) -> Vec<CocUpdate> {
    // All five sources fetched concurrently
    let (mut sc_news, mut sc_events, mut sc_community, mut youtube, mut reddit) = tokio::join!(
        fetch_rss(client, SUPERCELL_NEWS_RSS,      "Supercell News",      false, true),
        fetch_rss(client, SUPERCELL_EVENTS_RSS,    "Supercell Events",    false, true),
        fetch_rss(client, SUPERCELL_COMMUNITY_RSS, "Supercell Community", false, true),
        fetch_rss(client, COC_YOUTUBE_RSS,         "CoC YouTube",         true,  false),
        fetch_rss(client, REDDIT_OFFICIAL_RSS,     "r/ClashOfClans",      false, false),
    );

    let mut all: Vec<CocUpdate> = Vec::new();
    all.append(&mut sc_news);
    all.append(&mut sc_events);
    all.append(&mut sc_community);
    all.append(&mut youtube);
    all.append(&mut reddit);

    // Deduplicate by generated ID
    let mut seen = std::collections::HashSet::new();
    all.retain(|u| seen.insert(u.id.clone()));

    debug!("CoC fetcher: {} unique updates ready to post", all.len());
    all
}
