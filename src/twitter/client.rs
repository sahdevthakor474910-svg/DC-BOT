use anyhow::Result;
use reqwest::Client;
use tracing::{info, warn};

use super::models::Tweet;

/// The two X accounts to monitor — hardcoded as requested.
pub const ACCOUNTS: &[(&str, &str)] = &[
    ("dmc_poc",    "🌍 DMC Global"),
    ("dmc_poc_jp", "🌏 DMC Asia/JP"),
];

/// Simple HTML tag / entity stripper to format plaintext tweet body.
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

pub struct TwitterClient {
    http: Client,
}

impl TwitterClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self { http })
    }

    /// Expose the inner HTTP client for translation or other HTTP queries.
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// Fetch the latest tweets for `username`. Uses active Nitter RSS instances.
    pub async fn fetch_tweets(&self, username: &str, limit: usize) -> Result<Vec<Tweet>> {
        let instances = &[
            "https://nitter.net",
            "https://nitter.privacyredirect.com",
            "https://nitter.poast.org",
            "https://xcancel.com",
        ];

        let mut last_err = None;

        for &base_url in instances {
            let url = format!("{}/{}/rss", base_url, username);
            info!("🐦 Attempting to fetch tweets from Nitter RSS: {}", url);
            
            let res = match self.http.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("Connection error: {}", e));
                    continue;
                }
            };

            let status = res.status();
            if !status.is_success() {
                last_err = Some(anyhow::anyhow!("HTTP error {}: {}", status, res.text().await.unwrap_or_default()));
                continue;
            }

            let bytes = match res.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("Bytes read error: {}", e));
                    continue;
                }
            };

            let feed = match feed_rs::parser::parse(bytes.as_ref()) {
                Ok(f) => f,
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("RSS parse error: {}", e));
                    continue;
                }
            };

            // Check if it's the whitelisting warning or bot verification page
            let title = feed.title.as_ref().map(|t| t.content.as_str()).unwrap_or("");
            if title.contains("not yet whitelisted") || title.contains("bot") || title.contains("Loading") {
                last_err = Some(anyhow::anyhow!("Instance returned bot/whitelist challenge page"));
                continue;
            }

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

                let link = format!("https://twitter.com/{}/status/{}", username, tweet_id);

                let pub_date = entry.published
                    .map(|dt| dt.format("%b %d, %Y • %H:%M").to_string())
                    .unwrap_or_default();

                tweets.push(Tweet {
                    id: tweet_id,
                    account: username.to_string(),
                    text,
                    link,
                    pub_date,
                    published_at: entry.published,
                    translated_text: None,
                });

                if tweets.len() >= limit {
                    break;
                }
            }

            // Successfully fetched and parsed the tweets!
            return Ok(tweets);
        }

        Err(anyhow::anyhow!(
            "All Nitter instances failed to fetch tweets. Last error: {}",
            last_err.unwrap_or_else(|| anyhow::anyhow!("No instances configured"))
        ))
    }
}
