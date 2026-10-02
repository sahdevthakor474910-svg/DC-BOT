use base64::prelude::*;
use poise::serenity_prelude as serenity;
use reqwest::Client;
use tracing::debug;

use super::models::GeminiInlineData;

/// Extract and download up to 3 images/GIFs from a message or its referenced message.
pub async fn extract_media_from_message(
    http: &Client,
    message: &serenity::Message,
) -> Vec<GeminiInlineData> {
    let mut results = Vec::new();

    // 1. Check current message attachments
    for att in &message.attachments {
        if is_supported_media(&att.content_type, &att.filename) && att.size < 12_000_000 {
            if let Some(inline) = download_media_as_inline(http, &att.url, att.content_type.as_deref()).await {
                results.push(inline);
                if results.len() >= 3 {
                    return results;
                }
            }
        }
    }

    // 2. Check current message embeds (e.g. Tenor/Giphy GIF embeds or image embeds)
    for embed in &message.embeds {
        if let Some(img) = &embed.image {
            if let Some(inline) = download_media_as_inline(http, &img.url, None).await {
                results.push(inline);
                if results.len() >= 3 {
                    return results;
                }
            }
        } else if let Some(vid) = &embed.video {
            if let Some(inline) = download_media_as_inline(http, &vid.url, None).await {
                results.push(inline);
                if results.len() >= 3 {
                    return results;
                }
            }
        } else if let Some(thumb) = &embed.thumbnail {
            if let Some(inline) = download_media_as_inline(http, &thumb.url, None).await {
                results.push(inline);
                if results.len() >= 3 {
                    return results;
                }
            }
        }
    }

    // 3. Check for links in message content (Tenor, Giphy, direct image/GIF links)
    for word in message.content.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            if let Some(resolved_url) = resolve_media_link(http, word).await {
                if let Some(inline) = download_media_as_inline(http, &resolved_url, None).await {
                    results.push(inline);
                    if results.len() >= 3 {
                        return results;
                    }
                }
            }
        }
    }

    // 4. If nothing found on current message, check referenced message (reply context)
    if results.is_empty() {
        if let Some(ref_msg) = &message.referenced_message {
            for att in &ref_msg.attachments {
                if is_supported_media(&att.content_type, &att.filename) && att.size < 12_000_000 {
                    if let Some(inline) = download_media_as_inline(http, &att.url, att.content_type.as_deref()).await {
                        results.push(inline);
                        if results.len() >= 3 {
                            return results;
                        }
                    }
                }
            }

            for embed in &ref_msg.embeds {
                if let Some(img) = &embed.image {
                    if let Some(inline) = download_media_as_inline(http, &img.url, None).await {
                        results.push(inline);
                        if results.len() >= 3 {
                            return results;
                        }
                    }
                } else if let Some(vid) = &embed.video {
                    if let Some(inline) = download_media_as_inline(http, &vid.url, None).await {
                        results.push(inline);
                        if results.len() >= 3 {
                            return results;
                        }
                    }
                } else if let Some(thumb) = &embed.thumbnail {
                    if let Some(inline) = download_media_as_inline(http, &thumb.url, None).await {
                        results.push(inline);
                        if results.len() >= 3 {
                            return results;
                        }
                    }
                }
            }

            for word in ref_msg.content.split_whitespace() {
                if word.starts_with("http://") || word.starts_with("https://") {
                    if let Some(resolved_url) = resolve_media_link(http, word).await {
                        if let Some(inline) = download_media_as_inline(http, &resolved_url, None).await {
                            results.push(inline);
                            if results.len() >= 3 {
                                return results;
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

fn is_supported_media(content_type: &Option<String>, filename: &str) -> bool {
    if let Some(ct) = content_type {
        if ct.starts_with("image/") || ct.starts_with("video/mp4") || ct.starts_with("video/webm") {
            return true;
        }
    }
    let lower = filename.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".mp4")
        || lower.ends_with(".mov")
}

async fn resolve_media_link(http: &Client, url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    if lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".mp4")
    {
        return Some(url.to_string());
    }

    if lower.contains("media.tenor.com") || lower.contains("media.giphy.com") {
        return Some(url.to_string());
    }

    // If Tenor link like https://tenor.com/view/... or Giphy
    if lower.contains("tenor.com/view/") || lower.contains("giphy.com/gifs/") {
        if let Ok(resp) = http.get(url).timeout(std::time::Duration::from_secs(5)).send().await {
            if let Ok(html) = resp.text().await {
                // Look for og:image or og:video content
                if let Some(pos) = html.find("property=\"og:image\" content=\"") {
                    let sub = &html[pos + 29..];
                    if let Some(end) = sub.find('"') {
                        return Some(sub[..end].to_string());
                    }
                }
                if let Some(pos) = html.find("property=\"og:video\" content=\"") {
                    let sub = &html[pos + 29..];
                    if let Some(end) = sub.find('"') {
                        return Some(sub[..end].to_string());
                    }
                }
            }
        }
    }

    None
}

async fn download_media_as_inline(
    http: &Client,
    url: &str,
    content_type_hint: Option<&str>,
) -> Option<GeminiInlineData> {
    debug!("Downloading media from {}", url);
    let resp = http
        .get(url)
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let raw_ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .or(content_type_hint)
        .unwrap_or("image/jpeg");

    let mime = if raw_ct.contains("gif") || url.contains(".gif") {
        "image/gif"
    } else if raw_ct.contains("png") || url.contains(".png") {
        "image/png"
    } else if raw_ct.contains("webp") || url.contains(".webp") {
        "image/webp"
    } else if raw_ct.contains("mp4") || url.contains(".mp4") {
        "video/mp4"
    } else {
        "image/jpeg"
    };

    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 12_000_000 {
        return None;
    }

    let b64 = BASE64_STANDARD.encode(&bytes);
    Some(GeminiInlineData {
        mime_type: mime.to_string(),
        data: b64,
    })
}
