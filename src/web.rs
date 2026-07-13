use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response, Html},
};
use tracing::{info, warn, error};

use crate::data::Data;

#[derive(serde::Deserialize)]
pub struct PlayQuery {
    pub url: String,    // Hex encoded original page URL
    pub source: String, // "jav", "porn", "okxxx"
    pub title: String,  // URL encoded page title
}

#[derive(serde::Deserialize)]
pub struct StreamQuery {
    pub url: String,    // Hex encoded direct MP4/CDN URL
}

/// Create the Axum Router populated with health checks and player/proxy endpoints.
pub fn create_router(data: Data) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(health_handler))
        .route("/play", axum::routing::get(play_handler))
        .route("/stream", axum::routing::get(stream_handler))
        .with_state(data)
}

async fn health_handler() -> &'static str {
    "Bot is active!"
}

/// Dynamic playback screen serving a custom video player.
async fn play_handler(
    State(_data): State<Data>,
    Query(query): Query<PlayQuery>,
) -> Response {
    // 1. Decode hex url to get page URL
    let page_url = match decode_hex(&query.url) {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid URL hex: {}", e)).into_response(),
    };

    info!("Resolving video stream for [source={}] url: {}", query.source, page_url);

    // 2. Resolve the direct MP4 URL based on the source
    let direct_mp4_url = match query.source.as_str() {
        "jav" => {
            let video_id = extract_eporner_id(&page_url).unwrap_or_else(|| page_url.clone());
            match crate::jav::client::EpornerClient::new() {
                Ok(client) => match client.get_mp4_url(&video_id).await {
                    Ok(u) => u,
                    Err(e) => {
                        error!("Failed to resolve JAV video URL pattern for {}: {}", video_id, e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to resolve JAV video URL: {}", e)).into_response();
                    }
                },
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create JAV client: {}", e)).into_response(),
            }
        }
        "porn" => {
            let video_id = extract_redtube_id(&page_url).unwrap_or_else(|| page_url.clone());
            match crate::porn::client::PornClient::new() {
                Ok(client) => match client.get_mp4_url(&video_id).await {
                    Ok(u) => u,
                    Err(e) => {
                        error!("Failed to resolve Redtube video pattern for {}: {}", video_id, e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to resolve Porn video URL: {}", e)).into_response();
                    }
                },
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create Porn client: {}", e)).into_response(),
            }
        }
        "okxxx" => {
            match crate::okxxx::client::OkXxxClient::new() {
                Ok(client) => match client.get_mp4_url(&page_url).await {
                    Ok(u) => u,
                    Err(e) => {
                        error!("Failed to resolve OK.XXX video pattern for {}: {}", page_url, e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to resolve OK.XXX video URL: {}", e)).into_response();
                    }
                },
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create OK.XXX client: {}", e)).into_response(),
            }
        }
        _ => {
            page_url.clone()
        }
    };

    // 3. Hex-encode the direct MP4 URL so it's safe to request through `/stream`
    let proxy_url_hex = encode_hex(&direct_mp4_url);
    let stream_url = format!("/stream?url={}", proxy_url_hex);

    // 4. Load the player.html template and inject values
    let template = include_str!("web/player.html");
    let html = template
        .replace("{title}", &query.title)
        .replace("{stream_url}", &stream_url);

    // 5. Render HTML response
    Html(html).into_response()
}

/// Media streaming proxy supporting Range requests (enabling video scrubbing).
async fn stream_handler(
    State(data): State<Data>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
) -> Response {
    // 1. Decode hex url
    let target_url = match decode_hex(&query.url) {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)).into_response(),
    };

    // 2. Prepare request to target URL
    let mut req_builder = data.http_client.get(&target_url);

    // 3. Forward Range header if present
    if let Some(range) = headers.get("range") {
        if let Ok(range_str) = range.to_str() {
            req_builder = req_builder.header("range", range_str);
        }
    }
    
    // Add Referer/User-Agent headers to bypass CDN/referer protection blocks
    if target_url.contains("eporner.com") {
        req_builder = req_builder.header("Referer", "https://www.eporner.com/");
    } else if target_url.contains("redtube.com") || target_url.contains("rdtcdn.com") {
        req_builder = req_builder.header("Referer", "https://www.redtube.com/");
    } else if target_url.contains("ok.xxx") {
        req_builder = req_builder.header("Referer", "https://ok.xxx/");
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
    let status_u16 = upstream_res.status().as_u16();
    let status = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::OK);
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
            headers_mut.insert("content-type", HeaderValue::from_static("video/mp4"));
        }
        headers_mut.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    }

    // 7. Stream the body to avoid loading file in memory
    let stream = upstream_res.bytes_stream();
    let body = axum::body::Body::from_stream(stream);

    match response_builder.body(body) {
        Ok(res) => res,
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to construct streaming response: {}", e)).into_response(),
    }
}

// ─── Encoding Helpers ────────────────────────────────────────────────────────

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

fn extract_eporner_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"video-([a-zA-Z0-9]+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}

fn extract_redtube_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"redtube\.com/([0-9]+)").ok()?;
    re.captures(url).map(|cap| cap[1].to_string())
}
