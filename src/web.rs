use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};
use tracing::{info, warn};

use crate::data::Data;

#[derive(serde::Deserialize)]
pub struct PlayQuery {
    pub url: String,            // Hex-encoded page URL or direct video URL
    #[serde(default)]
    pub source: String,         // "jav", "porn", "okxxx", "hentai", "javhd", "xnxx", "direct", "pornclips"
    #[serde(default)]
    pub title: String,          // URL-encoded or raw title
}

#[derive(serde::Deserialize)]
pub struct StreamQuery {
    pub url: String,            // Hex-encoded direct MP4/CDN URL
}

/// Create the Axum Router with player and streaming proxy endpoints.
pub fn create_router(data: Data) -> axum::Router {
    axum::Router::new()
        .route("/play", axum::routing::get(play_handler))
        .route("/stream", axum::routing::get(stream_handler))
        .route("/stream/video.mp4", axum::routing::get(stream_handler))
        .with_state(data)
}

/// Dynamic playback screen serving the custom HTML5 video player with embed fallback.
pub async fn play_handler(
    State(_data): State<Data>,
    Query(query): Query<PlayQuery>,
) -> Response {
    // 1. Decode hex url to get target URL (can be page URL or already direct MP4)
    let raw_url = match decode_hex(&query.url) {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid URL hex: {}", e)).into_response(),
    };

    info!("Resolving video stream for [source={}] url: {}", query.source, raw_url);

    // Compute official embed URL as guaranteed fallback
    let embed_url = get_embed_url(&query.source, &raw_url);

    // 2. Resolve direct MP4/stream URL based on source (or use directly if already a media file)
    let direct_mp4_url = if raw_url.contains(".mp4") || raw_url.contains(".webm") || raw_url.contains(".m3u8") {
        raw_url.clone()
    } else {
        match query.source.as_str() {
            "jav" => {
                let video_id = extract_redtube_id(&raw_url).unwrap_or_else(|| raw_url.clone());
                match crate::porn::client::PornClient::new() {
                    Ok(client) => client.get_mp4_url(&video_id).await.unwrap_or_default(),
                    Err(_) => String::new(),
                }
            }
            "porn" => {
                let video_id = extract_redtube_id(&raw_url).unwrap_or_else(|| raw_url.clone());
                match crate::porn::client::PornClient::new() {
                    Ok(client) => client.get_mp4_url(&video_id).await.unwrap_or_default(),
                    Err(_) => String::new(),
                }
            }
            "okxxx" => {
                match crate::okxxx::client::OkXxxClient::new() {
                    Ok(client) => client.get_mp4_url(&raw_url).await.unwrap_or_default(),
                    Err(_) => String::new(),
                }
            }
            "hentai" | "hanime" => {
                match crate::hanime::client::HentaiClient::new() {
                    Ok(client) => client.get_mp4_url(&raw_url).await.unwrap_or_default(),
                    Err(_) => String::new(),
                }
            }
            "xnxx" => {
                match crate::xnxx::client::XnxxClient::new() {
                    Ok(client) => client.get_mp4_url(&raw_url).await.unwrap_or_default(),
                    Err(_) => String::new(),
                }
            }
            _ => {
                if raw_url.starts_with("http") {
                    raw_url.clone()
                } else {
                    String::new()
                }
            }
        }
    };

    // 3. Hex-encode the direct MP4 URL for the streaming proxy
    let stream_url = if !direct_mp4_url.is_empty() && direct_mp4_url.starts_with("http") {
        let proxy_url_hex = encode_hex(&direct_mp4_url);
        format!("/stream?url={}", proxy_url_hex)
    } else {
        String::new()
    };

    // 4. Load player.html template and inject values
    let template = include_str!("web/player.html");
    let html = template
        .replace("{title}", &query.title)
        .replace("{stream_url}", &stream_url)
        .replace("{direct_url}", &direct_mp4_url)
        .replace("{embed_url}", &embed_url);

    // 5. Render HTML response
    Html(html).into_response()
}

/// Media streaming proxy supporting Range requests (enabling video scrubbing).
pub async fn stream_handler(
    State(_data): State<Data>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
) -> Response {
    // 1. Decode hex url
    let target_url = match decode_hex(&query.url) {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)).into_response(),
    };

    // 2. Build dedicated client with browser User-Agent so CDNs don't block requests
    let client = match reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build client: {}", e)).into_response(),
    };

    let mut req_builder = client.get(&target_url)
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9");

    // 3. Forward Range header if present (crucial for video seeking)
    if let Some(range) = headers.get("range") {
        if let Ok(range_str) = range.to_str() {
            req_builder = req_builder.header("range", range_str);
        }
    }

    // Add Referer, Origin, and Sec-Fetch headers matching target CDN/platform (must be same-origin to prevent anti-hotlink placeholder)
    if target_url.contains("eporner") {
        req_builder = req_builder
            .header("Referer", "https://www.eporner.com/")
            .header("Origin", "https://www.eporner.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("redtube") || target_url.contains("rdtcdn") || target_url.contains("phncdn") || target_url.contains("pornhub") {
        req_builder = req_builder
            .header("Referer", "https://www.redtube.com/")
            .header("Origin", "https://www.redtube.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("ok.xxx") {
        req_builder = req_builder
            .header("Referer", "https://ok.xxx/")
            .header("Origin", "https://ok.xxx")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("hentaigasm") || target_url.contains("hgasm") {
        req_builder = req_builder
            .header("Referer", "https://hentaigasm.com/")
            .header("Origin", "https://hentaigasm.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("xnxx") {
        req_builder = req_builder
            .header("Referer", "https://www.xnxx.com/")
            .header("Origin", "https://www.xnxx.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("javhd") {
        req_builder = req_builder
            .header("Referer", "https://javhd.com/")
            .header("Origin", "https://javhd.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    } else if target_url.contains("redgifs") {
        req_builder = req_builder
            .header("Referer", "https://www.redgifs.com/")
            .header("Origin", "https://www.redgifs.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Dest", "video");
    }

    // 4. Send request to target media file
    let upstream_res = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Streaming connection to upstream URL {} failed: {}", target_url, e);
            return (StatusCode::BAD_GATEWAY, format!("Upstream connection failed: {}", e)).into_response();
        }
    };

    // 5. Build response copying status
    let status = StatusCode::from_u16(upstream_res.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut response_builder = Response::builder().status(status);

    // 6. Copy headers (Content-Type, Content-Length, Content-Range, Accept-Ranges)
    let upstream_headers = upstream_res.headers();
    let headers_to_forward = [
        "content-type",
        "content-length",
        "content-range",
        "accept-ranges",
    ];

    if let Some(headers_mut) = response_builder.headers_mut() {
        for h in headers_to_forward {
            if let Some(val) = upstream_headers.get(h) {
                if let Ok(axum_val) = HeaderValue::from_bytes(val.as_bytes()) {
                    if let Ok(name) = axum::http::HeaderName::from_bytes(h.as_bytes()) {
                        headers_mut.insert(name, axum_val);
                    }
                }
            }
        }
        if headers_mut.get("content-type").is_none() {
            if target_url.contains(".m3u8") {
                headers_mut.insert("content-type", HeaderValue::from_static("application/vnd.apple.mpegurl"));
            } else {
                headers_mut.insert("content-type", HeaderValue::from_static("video/mp4"));
            }
        }
        if headers_mut.get("accept-ranges").is_none() {
            headers_mut.insert("accept-ranges", HeaderValue::from_static("bytes"));
        }
        headers_mut.insert("access-control-allow-origin", HeaderValue::from_static("*"));
        headers_mut.insert("access-control-allow-headers", HeaderValue::from_static("*"));
    }

    // 7. Stream the body
    let stream = upstream_res.bytes_stream();
    let body = axum::body::Body::from_stream(stream);

    match response_builder.body(body) {
        Ok(res) => res,
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to construct streaming response: {}", e)).into_response(),
    }
}

// ─── Encoding & Embed Helpers ────────────────────────────────────────────────

pub fn encode_hex(s: &str) -> String {
    s.bytes().map(|b| format!("{:02x}", b)).collect()
}

pub fn decode_hex(s: &str) -> Result<String, String> {
    if s.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    let mut res = Vec::new();
    for i in (0..s.len()).step_by(2) {
        let byte_str = &s[i..i+2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|e| e.to_string())?;
        res.push(byte);
    }
    String::from_utf8(res).map_err(|e| e.to_string())
}

pub fn extract_eporner_id(url: &str) -> Option<String> {
    if !url.contains("http") && !url.contains('/') {
        return Some(url.to_string());
    }
    if let Ok(re) = regex::Regex::new(r"video-([a-zA-Z0-9]+)") {
        if let Some(cap) = re.captures(url) {
            return Some(cap[1].to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r"/([0-9]{6,10})(?:-[0-9]+p)?\.mp4") {
        if let Some(cap) = re.captures(url) {
            return Some(cap[1].to_string());
        }
    }
    None
}

pub fn extract_redtube_id(url: &str) -> Option<String> {
    if !url.contains("http") && !url.contains('/') {
        return Some(url.to_string());
    }
    if let Ok(re) = regex::Regex::new(r"redtube\.com/([0-9]+)") {
        if let Some(cap) = re.captures(url) {
            return Some(cap[1].to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r"[/?=]([0-9]{7,10})[/?&_.]") {
        if let Some(cap) = re.captures(url) {
            return Some(cap[1].to_string());
        }
    }
    None
}

pub fn extract_okxxx_id(url: &str) -> Option<String> {
    if !url.contains("http") && !url.contains('/') {
        return Some(url.to_string());
    }
    let re = regex::Regex::new(r"ok\.xxx/(?:video/|embed/)?([0-9]+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}

pub fn extract_xnxx_id(url: &str) -> Option<String> {
    if !url.contains("http") && !url.contains('/') {
        return Some(url.to_string());
    }
    let re = regex::Regex::new(r"video-([a-zA-Z0-9]+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}

pub fn extract_redgifs_id(url: &str) -> Option<String> {
    if !url.contains("http") && !url.contains('/') {
        return Some(url.to_string());
    }
    let re = regex::Regex::new(r"redgifs\.com/(?:watch/|ifr/)?([a-zA-Z0-9]+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}

pub fn get_embed_url(source: &str, raw_url: &str) -> String {
    match source {
        "okxxx" => {
            if let Some(id) = extract_okxxx_id(raw_url) {
                format!("https://ok.xxx/embed/{}", id)
            } else {
                raw_url.to_string()
            }
        }
        "porn" => {
            if let Some(id) = extract_redtube_id(raw_url) {
                format!("https://embed.redtube.com/?id={}", id)
            } else {
                raw_url.to_string()
            }
        }
        "jav" => {
            if let Some(id) = extract_redtube_id(raw_url) {
                format!("https://embed.redtube.com/?id={}", id)
            } else if let Some(id) = extract_eporner_id(raw_url) {
                format!("https://www.eporner.com/embed/{}/", id)
            } else {
                raw_url.to_string()
            }
        }
        "xnxx" => {
            if let Some(id) = extract_xnxx_id(raw_url) {
                format!("https://www.xnxx.com/embedframe/{}", id)
            } else {
                raw_url.to_string()
            }
        }
        "pornclips" | "redgifs" => {
            if let Some(id) = extract_redgifs_id(raw_url) {
                format!("https://www.redgifs.com/ifr/{}", id)
            } else {
                raw_url.to_string()
            }
        }
        _ => raw_url.to_string(),
    }
}
