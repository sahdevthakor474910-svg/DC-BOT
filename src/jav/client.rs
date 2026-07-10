use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use super::models::JavTitle;

// ─── R18.dev API response types ───────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct R18Response {
    #[serde(default)]
    result: Vec<R18Movie>,
}

#[derive(Deserialize, Debug)]
struct R18Movie {
    content_id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    #[serde(default)]
    actresses: Vec<R18Actress>,
    maker: Option<R18Maker>,
    images: Option<R18Images>,
    release_date: Option<String>,
}

#[derive(Deserialize, Debug)]
struct R18Actress {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct R18Maker {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct R18Images {
    jacket_image: Option<R18JacketImage>,
}

#[derive(Deserialize, Debug)]
struct R18JacketImage {
    medium: Option<String>,
    large: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────

/// Fetch latest JAV titles from R18.dev (sorted by newest release date).
pub async fn fetch_latest(client: &Client, count: usize) -> Result<Vec<JavTitle>> {
    fetch_r18(client, "release_date", count, false).await
}

/// Fetch popular JAV titles from R18.dev (sorted by monthly popularity).
pub async fn fetch_popular(client: &Client, count: usize) -> Result<Vec<JavTitle>> {
    fetch_r18(client, "ranking", count, true).await
}

/// Core R18.dev API fetch.
/// API docs: https://r18.dev/videos/vod/movies/list/...
async fn fetch_r18(
    client: &Client,
    sort: &str,
    count: usize,
    is_popular: bool,
) -> Result<Vec<JavTitle>> {
    let url = format!(
        "https://r18.dev/videos/vod/movies/list/-/director=-/actress=-/genre=-/studio=-/label=-/series=-/channel=-/type=A/id=1/sort={}/page=1/.json",
        sort
    );

    debug!("Fetching R18 JAV list (sort={}): {}", sort, url);

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; JAV-Bot/1.0)")
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json::<R18Response>()
        .await?;

    let titles: Vec<JavTitle> = resp
        .result
        .into_iter()
        .take(count)
        .filter_map(|m| {
            let content_id = m.content_id?;
            let title = m.title?;
            let url = m.url.unwrap_or_else(|| {
                format!("https://r18.dev/videos/vod/movies/detail/-/dvd_id={}/", content_id)
            });

            let cover_url = m.images.and_then(|img| {
                img.jacket_image.and_then(|j| j.large.or(j.medium))
            });

            let actresses: Vec<String> = m
                .actresses
                .into_iter()
                .filter_map(|a| a.name)
                .collect();

            let studio = m.maker.and_then(|mk| mk.name);

            // Trim release date to just YYYY-MM-DD
            let release_date = m.release_date.map(|d| {
                d.split_whitespace().next().unwrap_or(&d).to_string()
            });

            Some(JavTitle {
                content_id,
                title,
                url,
                cover_url,
                actresses,
                studio,
                release_date,
                is_popular,
            })
        })
        .collect();

    debug!("Got {} JAV titles (sort={})", titles.len(), sort);
    Ok(titles)
}

/// Fetch both latest and popular, deduplicated by content_id.
pub async fn fetch_all(client: &Client) -> Vec<JavTitle> {
    let mut all: Vec<JavTitle> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Latest releases — 6 titles
    match fetch_latest(client, 6).await {
        Ok(items) => {
            for item in items {
                if seen_ids.insert(item.content_id.clone()) {
                    all.push(item);
                }
            }
        }
        Err(e) => warn!("JAV latest fetch failed: {:#}", e),
    }

    // Popular titles — 4 titles
    match fetch_popular(client, 4).await {
        Ok(items) => {
            for item in items {
                if seen_ids.insert(item.content_id.clone()) {
                    all.push(item);
                }
            }
        }
        Err(e) => warn!("JAV popular fetch failed: {:#}", e),
    }

    all
}
