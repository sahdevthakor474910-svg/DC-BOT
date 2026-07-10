use anyhow::Result;
use feed_rs::parser;
use tracing::{debug, warn};

use super::models::NewsArticle;

/// Gaming news RSS feeds to poll.
pub const FEEDS: &[(&str, &str)] = &[
    ("GamesRadar",        "https://www.gamesradar.com/rss/"),
    ("Eurogamer",         "https://www.eurogamer.net/?format=rss"),
    ("Rock Paper Shotgun","https://www.rockpapershotgun.com/feed"),
    ("PC Gamer",          "https://www.pcgamer.com/rss/"),
    ("VG247",             "https://www.vg247.com/feed"),
];

/// Simple HTML tag / entity stripper (avoids html2text API churn).
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

/// Truncate a string to `max` chars, appending "…" if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Truncate at char boundary
        let end = s
            .char_indices()
            .map(|(i, _)| i)
            .take(max - 1)
            .last()
            .unwrap_or(max - 1);
        format!("{}…", &s[..end])
    }
}

/// Fetch every configured feed and merge into one list.
pub async fn fetch_all_feeds(client: &reqwest::Client) -> Vec<NewsArticle> {
    let mut articles = Vec::new();

    for (source, url) in FEEDS {
        match fetch_feed(client, url, source).await {
            Ok(mut batch) => {
                debug!("Got {} articles from {}", batch.len(), source);
                articles.append(&mut batch);
            }
            Err(e) => {
                warn!("Feed {} ({}) failed: {:#}", source, url, e);
            }
        }
    }

    articles
}

/// Fetch and parse a single RSS/Atom feed.
pub async fn fetch_feed(
    client: &reqwest::Client,
    url:    &str,
    source: &str,
) -> Result<Vec<NewsArticle>> {
    let bytes = client
        .get(url)
        .header(
            "Accept",
            "application/rss+xml, application/atom+xml, application/xml, text/xml, */*",
        )
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let feed = parser::parse(bytes.as_ref())
        .map_err(|e| anyhow::anyhow!("Parse error for {}: {}", url, e))?;

    let articles = feed
        .entries
        .into_iter()
        .take(5) // newest 5 per feed per tick
        .filter_map(|entry| {
            // Extract all fields BEFORE any field is moved
            let title     = entry.title.as_ref()?.content.clone();
            let link      = entry.links.first()?.href.clone();
            let summary   = entry.summary.map(|t| t.content);
            let content   = entry.content.and_then(|c| c.body);
            let published = entry.published;
            let media     = entry.media;

            if link.is_empty() {
                return None;
            }

            let description = summary
                .or(content)
                .map(|html| truncate(&strip_html(&html), 200));

            // Try to extract an image from media objects
            let image_url = media.iter().find_map(|m| {
                m.content.iter().find_map(|c| {
                    c.url.as_ref().map(|u| u.as_str().to_string())
                })
            });

            let id = format!("news::{}::{}", source, link);

            Some(NewsArticle {
                id,
                title,
                url: link,
                description,
                image_url,
                source: source.to_string(),
                published_at: published,
            })
        })
        .collect();


    Ok(articles)
}
