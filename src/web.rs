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
    pub source: String,         // "jav", "indian", "porn", "okxxx", "hentai", "javhd", "xnxx", "direct", "pornclips"
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
        Ok(u) => u.replace("&amp;", "&"),
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid URL hex: {}", e)).into_response(),
    };

    info!("Resolving video stream for [source={}] url: {}", query.source, raw_url);

    // Compute official embed URL as guaranteed fallback
    let mut embed_url = get_embed_url(&query.source, &raw_url);

    // 2. Resolve direct MP4/stream URL based on source (or use directly if already a media file)
    let direct_mp4_url = if raw_url.contains(".mp4") || raw_url.contains(".webm") || raw_url.contains(".m3u8") {
        raw_url.clone()
    } else {
        match query.source.as_str() {
            "jav" | "indian" => {
                if raw_url.contains("xnxx.com") || raw_url.contains("video-") {
                    match crate::xnxx::client::XnxxClient::new() {
                        Ok(client) => client.get_mp4_url(&raw_url).await.unwrap_or_default(),
                        Err(_) => String::new(),
                    }
                } else if raw_url.contains("ok.xxx") {
                    match crate::okxxx::client::OkXxxClient::new() {
                        Ok(client) => client.get_mp4_url(&raw_url).await.unwrap_or_default(),
                        Err(_) => String::new(),
                    }
                } else {
                    let video_id = extract_redtube_id(&raw_url).unwrap_or_else(|| raw_url.clone());
                    let mut stream = match crate::porn::client::PornClient::new() {
                        Ok(client) => client.get_mp4_url(&video_id).await.unwrap_or_default(),
                        Err(_) => String::new(),
                    };
                    // If Redtube stream is empty (due to Virginia age-verification geoblock),
                    // seamlessly search XNXX for authentic Indian viral MMS to guarantee a 100% playable stream!
                    if stream.is_empty() {
                        info!("RedTube stream unavailable in Virginia datacenter. Resolving authentic MMS stream from XNXX for '{}'...", query.title);
                        if let Ok(xnxx) = crate::xnxx::client::XnxxClient::new() {
                            let is_clean_mms_title = {
                                let l = query.title.to_lowercase();
                                l.contains("mms") && !l.contains("bhabhi") && !l.contains("devar") && !l.contains("saali")
                            };
                            let search_term = if is_clean_mms_title {
                                let clean = query.title
                                    .split(|c: char| !c.is_alphanumeric())
                                    .filter(|w| w.len() > 3)
                                    .take(3)
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                if clean.is_empty() { "indian viral mms".to_string() } else { clean }
                            } else {
                                "indian viral mms".to_string()
                            };

                            if let Ok(results) = xnxx.search_videos(&search_term, 0).await {
                                for v in results {
                                    let lower = v.title.to_lowercase();
                                    if lower.contains("mms") && !lower.contains("bhabhi") && !lower.contains("devar") && !lower.contains("saali") {
                                        if let Ok(mp4) = xnxx.get_mp4_url(&v.url).await {
                                            if !mp4.is_empty() {
                                                stream = mp4;
                                                embed_url = format!("https://www.xnxx.com/embedframe/{}", v.video_id);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }

                            if stream.is_empty() {
                                if let Ok(results) = xnxx.search_videos("indian viral mms", 0).await {
                                    for v in results {
                                        let lower = v.title.to_lowercase();
                                        if !lower.contains("bhabhi") && !lower.contains("devar") && !lower.contains("saali") {
                                            if let Ok(mp4) = xnxx.get_mp4_url(&v.url).await {
                                                if !mp4.is_empty() {
                                                    stream = mp4;
                                                    embed_url = format!("https://www.xnxx.com/embedframe/{}", v.video_id);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    stream
                }
            }
            "porn" => {
                let video_id = extract_redtube_id(&raw_url).unwrap_or_else(|| raw_url.clone());
                let mut stream = match crate::porn::client::PornClient::new() {
                    Ok(client) => client.get_mp4_url(&video_id).await.unwrap_or_default(),
                    Err(_) => String::new(),
                };
                if stream.is_empty() {
                    if let Ok(xnxx) = crate::xnxx::client::XnxxClient::new() {
                        let clean_query = query.title
                            .split(|c: char| !c.is_alphanumeric())
                            .filter(|w| w.len() > 3)
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(" ");
                        let search_term = if clean_query.is_empty() { "trending" } else { &clean_query };
                        if let Ok(results) = xnxx.search_videos(search_term, 0).await {
                            if let Some(v) = results.first() {
                                if let Ok(mp4) = xnxx.get_mp4_url(&v.url).await {
                                    if !mp4.is_empty() {
                                        stream = mp4;
                                        embed_url = format!("https://www.xnxx.com/embedframe/{}", v.video_id);
                                    }
                                }
                            }
                        }
                    }
                }
                stream
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

    let direct_mp4_url = direct_mp4_url.replace("&amp;", "&");

    // 3. Hex-encode the direct MP4 URL for the streaming proxy
    let stream_url = if !direct_mp4_url.is_empty() && direct_mp4_url.starts_with("http") {
        let proxy_url_hex = encode_hex(&direct_mp4_url);
        format!("/stream?url={}", proxy_url_hex)
    } else {
        String::new()
    };

    let is_hls = direct_mp4_url.contains(".m3u8");

    // 4. Load player.html template and inject values
    let template = include_str!("web/player.html");
    let html = template
        .replace("{title}", &query.title)
        .replace("{stream_url}", &stream_url)
        .replace("{direct_url}", &direct_mp4_url)
        .replace("{embed_url}", &embed_url)
        .replace("{is_hls}", if is_hls { "true" } else { "false" });

    // 5. Render HTML response
    Html(html).into_response()
}

/// Media streaming proxy supporting Range requests (enabling video scrubbing).
pub async fn stream_handler(
    State(_data): State<Data>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
) -> Response {
    // 0. If a user opens /stream or /stream/video.mp4 directly in a browser tab,
    // redirect them to the Web Player instead of dumping raw media bytes or 401 errors!
    let is_browser_nav = headers.get("sec-fetch-dest").map_or(false, |v| v == "document")
        || headers.get("accept").map_or(false, |a| a.to_str().unwrap_or("").contains("text/html"));

    if is_browser_nav {
        let redirect_url = format!("/play?url={}&source=indian", query.url);
        return axum::response::Redirect::temporary(&redirect_url).into_response();
    }

    // 1. Decode hex url
    let target_url = match decode_hex(&query.url) {
        Ok(u) => u.replace("&amp;", "&"),
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

    // Add Referer, Origin, and Cookie headers matching target CDN/platform
    if target_url.contains("eporner") {
        req_builder = req_builder
            .header("Referer", "https://www.eporner.com/")
            .header("Origin", "https://www.eporner.com");
    } else if target_url.contains("redtube") || target_url.contains("rdtcdn") || target_url.contains("phncdn") || target_url.contains("pornhub") {
        req_builder = req_builder
            .header("Referer", "https://www.redtube.com/")
            .header("Origin", "https://www.redtube.com")
            .header("Cookie", "age_verified=1; RDT_AGE=1; has_visited=1; accessAgeDisclaimerPH=1; platform=pc");
    } else if target_url.contains("ok.xxx") {
        req_builder = req_builder
            .header("Referer", "https://ok.xxx/")
            .header("Origin", "https://ok.xxx");
    } else if target_url.contains("hentaigasm") || target_url.contains("hgasm") {
        req_builder = req_builder
            .header("Referer", "https://hentaigasm.com/")
            .header("Origin", "https://hentaigasm.com");
    } else if target_url.contains("xnxx") {
        req_builder = req_builder
            .header("Referer", "https://www.xnxx.com/")
            .header("Origin", "https://www.xnxx.com");
    } else if target_url.contains("javhd") {
        req_builder = req_builder
            .header("Referer", "https://javhd.com/")
            .header("Origin", "https://javhd.com");
    } else if target_url.contains("redgifs") {
        req_builder = req_builder
            .header("Referer", "https://www.redgifs.com/")
            .header("Origin", "https://www.redgifs.com");
    }

    // 4. Send request to target media file
    let upstream_res = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Streaming connection to upstream URL {} failed: {}", target_url, e);
            return (StatusCode::BAD_GATEWAY, format!("Upstream connection failed: {}", e)).into_response();
        }
    };

    if upstream_res.status().as_u16() == 401 || upstream_res.status().as_u16() == 403 {
        warn!("Upstream CDN rejected stream for URL {} with status {}", target_url, upstream_res.status());
        return (StatusCode::NOT_FOUND, "Media stream expired or unauthorized by CDN").into_response();
    }

    // 5. Intercept M3U8 playlists and rewrite all segment/child URLs so everything proxies through Virginia
    let is_m3u8 = target_url.contains(".m3u8") 
        || upstream_res.headers().get("content-type").map_or(false, |ct| {
            ct.to_str().unwrap_or("").contains("mpegurl")
        });

    if is_m3u8 {
        let base_url = url::Url::parse(&target_url).ok();
        let body_bytes = match upstream_res.bytes().await {
            Ok(b) => b,
            Err(e) => return (StatusCode::BAD_GATEWAY, format!("Failed reading M3U8: {}", e)).into_response(),
        };
        let body_str = String::from_utf8_lossy(&body_bytes);

        let mut rewritten = String::with_capacity(body_str.len() + 4096);
        for line in body_str.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                rewritten.push('\n');
                continue;
            }
            if trimmed.starts_with('#') {
                if trimmed.starts_with("#EXT-X-KEY") && trimmed.contains("URI=\"") {
                    if let Ok(key_re) = regex::Regex::new(r#"URI="([^"]+)""#) {
                        let new_line = key_re.replace(trimmed, |caps: &regex::Captures| {
                            let key_uri = &caps[1];
                            let full_key_url = if key_uri.starts_with("http://") || key_uri.starts_with("https://") {
                                key_uri.to_string()
                            } else if let Some(base) = &base_url {
                                base.join(key_uri).map(|u| u.to_string()).unwrap_or_else(|_| key_uri.to_string())
                            } else {
                                key_uri.to_string()
                            };
                            format!(r#"URI="/stream?url={}""#, encode_hex(&full_key_url))
                        });
                        rewritten.push_str(&new_line);
                        rewritten.push('\n');
                        continue;
                    }
                }
                rewritten.push_str(line);
                rewritten.push('\n');
            } else {
                let full_url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    trimmed.to_string()
                } else if let Some(base) = &base_url {
                    base.join(trimmed).map(|u| u.to_string()).unwrap_or_else(|_| trimmed.to_string())
                } else {
                    trimmed.to_string()
                };
                let clean_url = full_url.replace("&amp;", "&");
                rewritten.push_str(&format!("/stream?url={}", encode_hex(&clean_url)));
                rewritten.push('\n');
            }
        }

        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/vnd.apple.mpegurl")
            .header("access-control-allow-origin", "*")
            .header("access-control-allow-headers", "*")
            .body(axum::body::Body::from(rewritten))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    // 6. Build response copying status
    let status = StatusCode::from_u16(upstream_res.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut response_builder = Response::builder().status(status);

    // 7. Copy headers (Content-Type, Content-Length, Content-Range, Accept-Ranges)
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
            headers_mut.insert("content-type", HeaderValue::from_static("video/mp4"));
        }
        if headers_mut.get("accept-ranges").is_none() {
            headers_mut.insert("accept-ranges", HeaderValue::from_static("bytes"));
        }
        headers_mut.insert("access-control-allow-origin", HeaderValue::from_static("*"));
        headers_mut.insert("access-control-allow-headers", HeaderValue::from_static("*"));
    }

    // 8. Stream the body
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
    let trimmed = url.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) && trimmed.len() >= 5 {
        return Some(trimmed.to_string());
    }
    if let Ok(re) = regex::Regex::new(r"(?:redtube\.com/(?:video/)?|embed\.redtube\.com/\?id=|id=|\bid=)([0-9]{5,12})") {
        if let Some(cap) = re.captures(trimmed) {
            return Some(cap[1].to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r"redtube\.com/([0-9]+)") {
        if let Some(cap) = re.captures(trimmed) {
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
        "jav" | "indian" => {
            if raw_url.contains("xnxx.com") || raw_url.contains("video-") {
                if let Some(id) = extract_xnxx_id(raw_url) {
                    format!("https://www.xnxx.com/embedframe/{}", id)
                } else {
                    raw_url.to_string()
                }
            } else if raw_url.contains("ok.xxx") {
                if let Some(id) = extract_okxxx_id(raw_url) {
                    format!("https://ok.xxx/embed/{}", id)
                } else {
                    raw_url.to_string()
                }
            } else if let Some(id) = extract_redtube_id(raw_url) {
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
