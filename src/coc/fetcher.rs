use feed_rs::parser;
use tracing::{debug, info, warn};

use super::models::CocUpdate;

// ── Official sources ──────────────────────────────────────────────────────────
//
// 1. Supercell Inbox RSS (skrwo/supercell-inbox-rss on GitHub Pages)
//    — Scrapes Supercell's official in-game CMS news every hour.
//    — This is the same content shown in the CoC "News" tab in-game.
//    — Updated at minute :25 of every hour.
//
// 2. Official CoC YouTube channel (channel ID: UCD1Em4q9088Z-0G5Jq12d6g)
//    — Gives us trailers, developer updates, and season reveals.
//
// 3. r/ClashOfClans "Official" flair search RSS
//    — Supercell community managers post here (u/ClashOfClans, u/Darian_Supercell).
//    — Filtered to Official flair + sorted by new.

const SUPERCELL_INBOX_RSS: &str =
    "https://skrwo.github.io/supercell-inbox-rss/rss/clash-of-clans/en.xml";

const COC_YOUTUBE_RSS: &str =
    "https://www.youtube.com/feeds/videos.xml?channel_id=UCD1Em4q9088Z-0G5Jq12d6g";

const REDDIT_OFFICIAL_RSS: &str =
    "https://www.reddit.com/r/ClashOfClans/search.rss?q=flair%3A%22Official%22&sort=new&restrict_sr=1&t=week";

// Only accept posts newer than this many hours to avoid re-broadcasting old news.
const MAX_AGE_HOURS: i64 = 72;

// ── Keyword classifier ────────────────────────────────────────────────────────

/// Returns the emoji tag for a CoC post based on its title.
/// Returns None to silently drop irrelevant posts.
fn classify(title: &str) -> Option<&'static str> {
    let t = title.to_lowercase();

    // Supercell Inbox posts are always official — classify by content
    if t.contains("free") || t.contains("reward") || t.contains("gift")
        || t.contains("magic item") || t.contains("code") || t.contains("redeem")
        || t.contains("giveaway") || t.contains("gem") || t.contains("free gift")
    {
        return Some("🎁 Free Reward");
    }
    if t.contains("cwl") || t.contains("clan war league") || t.contains("war league") {
        return Some("🏆 Clan War League");
    }
    if t.contains("event") || t.contains("challenge") || t.contains("championship")
        || t.contains("esport") || t.contains("qualifier") || t.contains("legend league")
        || t.contains("gold pass") || t.contains("clan games") || t.contains("magic items")
        || t.contains("spotlight")
    {
        return Some("🏅 Event");
    }
    if t.contains("update") || t.contains("patch") || t.contains("season")
        || t.contains("sneak peek") || t.contains("balance") || t.contains("maintenance")
        || t.contains("new hero") || t.contains("new troop") || t.contains("new spell")
        || t.contains("town hall") || t.contains("builder hall") || t.contains("new feature")
        || t.contains("coming soon") || t.contains("teaser") || t.contains("changes")
    {
        return Some("⚔️ Update");
    }
    if t.contains("announcement") || t.contains("reveal") || t.contains("official")
        || t.contains("developer") || t.contains("community")
    {
        return Some("📢 Announcement");
    }

    // Accept anything from Supercell's own CMS — it's all official
    Some("📰 News")
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
        let end = s.char_indices().map(|(i, _)| i).take(max - 1).last().unwrap_or(max - 1);
        format!("{}…", &s[..end])
    }
}

/// Returns true if the item is fresh (within MAX_AGE_HOURS).
fn is_recent(published: Option<chrono::DateTime<chrono::Utc>>) -> bool {
    match published {
        Some(ts) => {
            let age = chrono::Utc::now() - ts;
            age.num_hours() <= MAX_AGE_HOURS
        }
        // If no date is provided, include it (Supercell Inbox may omit dates on old items)
        None => false,
    }
}

// ── Fetch helpers ─────────────────────────────────────────────────────────────

/// Generic RSS/Atom fetcher. `is_youtube` skips keyword classification.
/// `is_supercell_inbox` accepts all items regardless of keyword match.
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
            .header("User-Agent", "discord-coc-bot/2.0 (Clash watcher)")
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

            // Date gate: skip items older than MAX_AGE_HOURS
            if !is_recent(published) && !is_supercell_inbox {
                return None;
            }

            let tag = if is_youtube {
                "📺 Update Video"
            } else if is_supercell_inbox {
                // Supercell Inbox: always tag, accept all
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

            let id = format!("coc::{}::{}", source, link);

            Some(CocUpdate {
                id,
                title,
                description,
                url:        link,
                image_url,
                source:     source.to_string(),
                published_at: published,
                tag:        tag.to_string(),
            })
        }).collect();

        Ok(items)
    }.await;

    match result {
        Ok(items) => {
            info!("CoC {source}: {} item(s) fetched", items.len());
            items
        }
        Err(e) => {
            warn!("CoC {source} RSS failed: {:#}", e);
            vec![]
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub async fn fetch_all_updates(client: &reqwest::Client) -> Vec<CocUpdate> {
    // Fetch all three sources concurrently
    let (mut supercell, mut youtube, mut reddit) = tokio::join!(
        fetch_rss(client, SUPERCELL_INBOX_RSS, "Supercell Official", false, true),
        fetch_rss(client, COC_YOUTUBE_RSS,     "CoC YouTube",        true,  false),
        fetch_rss(client, REDDIT_OFFICIAL_RSS, "r/ClashOfClans",     false, false),
    );

    let mut all: Vec<CocUpdate> = Vec::new();
    all.append(&mut supercell);
    all.append(&mut youtube);
    all.append(&mut reddit);

    // Deduplicate by id
    let mut seen = std::collections::HashSet::new();
    all.retain(|u| seen.insert(u.id.clone()));

    debug!("CoC fetcher: {} total updates after dedup", all.len());
    all
}
