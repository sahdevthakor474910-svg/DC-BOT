use anyhow::{anyhow, Result};
use reqwest::Client;
use tracing::warn;

use super::models::Tweet;

/// The two X accounts to monitor — hardcoded as requested.
pub const ACCOUNTS: &[(&str, &str)] = &[
    ("dmc_poc",    "🌍 DMC Global"),
    ("dmc_poc_jp", "🌏 DMC Asia/JP"),
];

/// Nitter instances tried in order. If one fails, the next is attempted.
/// Public instances come and go; this list covers the most stable ones.
const NITTER_INSTANCES: &[&str] = &[
    "https://nitter.net",
    "https://nitter.poast.org",
    "https://nitter.cz",
    "https://nitter.it",
];

pub struct TwitterClient {
    http: Client,
}

impl TwitterClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(20))
            .build()?;
        Ok(Self { http })
    }

    /// Expose the inner HTTP client for translation or other HTTP queries.
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// Fetch the latest tweets for `username` by trying each Nitter instance
    /// until one succeeds. Returns at most `limit` tweets.
    pub async fn fetch_tweets(&self, username: &str, limit: usize) -> Result<Vec<Tweet>> {
        let mut last_err: Option<anyhow::Error> = None;

        for instance in NITTER_INSTANCES {
            let rss_url = format!("{}/{}/rss", instance, username);

            match self.fetch_rss(&rss_url, username, limit).await {
                Ok(tweets) => {
                    if !tweets.is_empty() {
                        return Ok(tweets);
                    }
                    // Empty feed — try next instance
                }
                Err(e) => {
                    warn!("Nitter instance {} failed for @{}: {}", instance, username, e);
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("All Nitter instances returned empty feeds for @{}", username)))
    }

    // ── Private ──────────────────────────────────────────────────────────────

    async fn fetch_rss(&self, url: &str, username: &str, limit: usize) -> Result<Vec<Tweet>> {
        let body = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let feed = feed_rs::parser::parse(body.as_ref())
            .map_err(|e| anyhow!("RSS parse error: {}", e))?;

        let mut tweets = Vec::new();

        for entry in feed.entries.into_iter().take(limit) {
            // ── Extract link ────────────────────────────────────────────────
            let nitter_link = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default();

            // Convert Nitter link → canonical twitter.com link
            let tweet_link = to_twitter_link(&nitter_link);

            // ── Extract tweet ID from the URL ────────────────────────────────
            let tweet_id = extract_tweet_id(&tweet_link)
                .unwrap_or_else(|| nitter_link.clone());

            if tweet_id.is_empty() {
                continue;
            }

            // ── Extract text ─────────────────────────────────────────────────
            // RSS title = stripped tweet summary; description = full HTML body.
            // Prefer the summary/description content with HTML stripped.
            let raw_text = entry
                .summary
                .as_ref()
                .map(|t| t.content.clone())
                .or_else(|| entry.title.as_ref().map(|t| t.content.clone()))
                .unwrap_or_default();

            let text = strip_html(&raw_text);

            // Skip retweets if you want — RT tweets start with "RT @"
            // (Uncomment below to filter them out)
            // if text.starts_with("RT @") { continue; }

            // ── Published date ───────────────────────────────────────────────
            let pub_date = entry
                .published
                .map(|dt| dt.format("%d %b %Y • %H:%M UTC").to_string())
                .unwrap_or_default();

            tweets.push(Tweet {
                id: tweet_id,
                account: username.to_string(),
                text,
                link: tweet_link,
                pub_date,
            });
        }

        Ok(tweets)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Replace a Nitter instance domain with twitter.com so the link opens on X.
fn to_twitter_link(nitter_url: &str) -> String {
    // e.g. https://nitter.privacydev.net/dmc_poc/status/123#m
    //   → https://twitter.com/dmc_poc/status/123
    let cleaned = nitter_url.trim_end_matches("#m");

    for instance in NITTER_INSTANCES {
        if cleaned.starts_with(instance) {
            return cleaned.replacen(instance, "https://twitter.com", 1);
        }
    }

    // Already canonical or unknown — return as-is
    cleaned.to_string()
}

/// Parse the numeric tweet ID from `https://twitter.com/<user>/status/<id>`.
fn extract_tweet_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"/status/(\d+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}

/// Strip HTML tags from a string and decode common HTML entities.
fn strip_html(html: &str) -> String {
    // Remove all <tag ...> blocks
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let no_tags = re.replace_all(html, "");

    // Decode basic HTML entities
    no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}
