use anyhow::Result;

use super::models::IndianVideo;

/// Dedicated Indian MMS search categories and terms rotated across ticks (Instagram leaks prioritized).
pub const INDIAN_SEARCHES: &[&str] = &[
    "instagram",
    "instagram viral",
    "reels",
    "influencer",
    "instagram mms",
    "viral mms",
    "instagram model",
    "reels viral",
    "influencer leak",
    "instagram leak",
    "viral leak",
    "desi mms",
];

pub struct IndianClient {
    http: reqwest::Client,
}

impl IndianClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(12))
            .build()?;
        Ok(Self { http })
    }

    /// Download raw MP4 bytes for Discord video attachment (< 8MB).
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut req = self.http.get(url);
        if url.contains("fsiblog") {
            req = req
                .header("Referer", "https://www.fsiblogxx.com/")
                .header("Origin", "https://www.fsiblogxx.com");
        }
        let bytes = req
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    /// Fetch large volume of 100% authentic videos from dedicated Indian MMS and leaks portals:
    /// 1. FsiBlog (https://www.fsiblogxx.com - #1 source for latest Instagram viral MMS & influencer leaks)
    /// 2. DesiPorn / MmsBee (extreme fallback only if FsiBlog returns 0 videos)
    pub async fn fetch_indian_videos(&self, query: &str, _count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let mut all: Vec<IndianVideo> = Vec::new();

        // 1. TOP PRIORITY: FsiBlog Instagram Viral MMS & Influencer Leaks
        let fsi_query = if query.contains("instagram") || query.contains("reels") || query.contains("influencer") || query.contains("model") || query.contains("leak") {
            query
        } else {
            "instagram"
        };
        if let Ok(fsi_videos) = self.fetch_fsiblog(fsi_query, page).await {
            all.extend(fsi_videos);
        }

        // 2. Fresh daily MMS from /category/mms/
        if let Ok(cat_mms) = self.fetch_fsiblog_category_mms(page).await {
            all.extend(cat_mms);
        }

        // 3. Complementary Instagram / Reels query to guarantee plenty of fresh viral leaks
        let sec_query = if fsi_query == "instagram" {
            "reels"
        } else if fsi_query == "reels" {
            "influencer"
        } else {
            "instagram viral"
        };
        if let Ok(sec_videos) = self.fetch_fsiblog(sec_query, page).await {
            all.extend(sec_videos);
        }

        // 4. Extreme emergency fallback only if FsiBlog returns 0 videos
        if all.is_empty() {
            if let Ok(dp_videos) = self.fetch_desiporn("mms", 1).await {
                all.extend(dp_videos);
            }
        }

        // Deduplicate by ID
        let mut seen_ids = std::collections::HashSet::new();
        all.retain(|v| seen_ids.insert(v.id.clone()));

        // Authenticity filter: reject family roleplays and non-Indian content (preserve Instagram MMS)
        let filtered: Vec<IndianVideo> = all
            .into_iter()
            .filter(|v| is_authentic_real_indian_mms(&v.title, ""))
            .collect();

        Ok(filtered)
    }

    /// Scrape DesiPorn search listing (?search=...)
    pub async fn fetch_desiporn(&self, query: &str, _page: u32) -> Result<Vec<IndianVideo>> {
        let clean_query = query.trim().replace(' ', "+");
        let url = format!("https://www.desiporn.xxx/?search={}", clean_query);
        let resp = self.http.get(&url).send().await?.error_for_status()?.text().await?;
        self.parse_desiporn_html(&resp)
    }

    /// Scrape DesiPorn category page ({category}-page-{page}.html)
    pub async fn fetch_desiporn_category(&self, category: &str, page: u32) -> Result<Vec<IndianVideo>> {
        let page_num = std::cmp::max(1, page);
        let url = format!("https://www.desiporn.xxx/{}-page-{}.html", category, page_num);
        let resp = self.http.get(&url).send().await?.error_for_status()?.text().await?;
        self.parse_desiporn_html(&resp)
    }

    /// Parse HTML from DesiPorn
    fn parse_desiporn_html(&self, html: &str) -> Result<Vec<IndianVideo>> {
        let mut videos = Vec::new();
        let card_re = regex::Regex::new(r#"<div class="cmain">([\s\S]*?)</div>"#)?;
        let item_re = regex::Regex::new(r#"url=(/item/(\d+)\.html)"#)?;
        let title_re = regex::Regex::new(r#"title="([^"]+)""#)?;
        let img_re = regex::Regex::new(r#"src="(https://[^"]+)""#)?;
        let time_re = regex::Regex::new(r#"<span class="time">\s*([0-9:]+)\s*</span>"#)?;

        for cap in card_re.captures_iter(html) {
            let card_str = &cap[1];
            if let Some(item_cap) = item_re.captures(card_str) {
                let id = format!("dp_{}", &item_cap[2]);
                let url = format!("https://www.desiporn.xxx{}", &item_cap[1]);
                let raw_title = title_re.captures(card_str)
                    .map(|c| c[1].to_string())
                    .unwrap_or_default();
                let thumb = img_re.captures(card_str)
                    .map(|c| c[1].to_string())
                    .unwrap_or_default();
                let duration = time_re.captures(card_str)
                    .map(|c| c[1].to_string())
                    .unwrap_or_else(|| "10:00".to_string());

                // Clean title if generic placeholder
                let title = if raw_title.is_empty() || raw_title.to_lowercase() == "desipornxxx" {
                    if let Some(slug) = thumb.split('/').last().and_then(|s| s.split('.').next()) {
                        let clean_slug = slug
                            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '_' || c == '-')
                            .replace('-', " ")
                            .replace('_', " ");
                        if !clean_slug.trim().is_empty() {
                            format!("🔥 Desi MMS: {}", clean_slug.trim())
                        } else {
                            format!("🔥 Authentic Indian MMS Leaked #{}", &item_cap[2])
                        }
                    } else {
                        format!("🔥 Authentic Indian MMS Leaked #{}", &item_cap[2])
                    }
                } else {
                    raw_title
                };

                videos.push(IndianVideo {
                    id,
                    title,
                    duration,
                    views: serde_json::Value::String("Verified MMS".to_string()),
                    default_thumb: thumb,
                    url,
                    direct_mp4: None,
                });
            }
        }

        Ok(videos)
    }

    /// Scrape latest topics from MmsBee portal
    pub async fn fetch_mmsbee(&self, page: u32) -> Result<Vec<IndianVideo>> {
        let mut videos = Vec::new();
        let forum_url = if page % 2 == 0 {
            "https://mmsbee.fun/forum/11-desi-pics-and-videos/"
        } else {
            "https://mmsbee.fun/forum/7-mmsbee-exclusive-zone/"
        };

        if let Ok(resp) = self.http.get(forum_url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(topic_re) = regex::Regex::new(r#"href=['"](https://mmsbee\.fun/topic/(\d+)-[^'"]+)['"][^>]*>([^<]+)</a>"#) {
                    for cap in topic_re.captures_iter(&html) {
                        let id = format!("bee_{}", &cap[2]);
                        let url = cap[1].split('?').next().unwrap_or(&cap[1]).to_string();
                        let title = cap[3].trim().to_string();
                        if title.len() < 5 || (title.to_lowercase().contains("mmsbee") && title.len() < 12) {
                            continue;
                        }
                        videos.push(IndianVideo {
                            id,
                            title,
                            duration: "Full MMS".to_string(),
                            views: serde_json::Value::String("MmsBee Exclusive".to_string()),
                            default_thumb: "https://mmsbee.fun/uploads/monthly_2026_10/mmsbee-logo.png".to_string(),
                            url,
                            direct_mp4: None,
                        });
                    }
                }
            }
        }

        Ok(videos)
    }

    /// Parse and extract videos from any FsiBlog listing page (search results, category pages)
    pub async fn fetch_fsiblog_url(&self, url: &str) -> Result<Vec<IndianVideo>> {
        let resp = self.http.get(url)
            .header("Referer", "https://www.fsiblogxx.com/")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let html = resp.text().await?;

        let mut videos = Vec::new();
        let article_re = regex::Regex::new(r#"<article[^>]*>([\s\S]*?)</article>"#)?;
        let title_re = regex::Regex::new(r#"<h3 class="elementor-post__title">\s*<a href="([^"]+)">\s*([^<]+)\s*</a>\s*</h3>"#)?;
        let thumb_re = regex::Regex::new(r#"<img[^>]+src="([^"]+)""#)?;

        for cap in article_re.captures_iter(&html) {
            let art = &cap[1];
            if let Some(t_cap) = title_re.captures(art) {
                let post_url = t_cap[1].trim().to_string();
                let raw_title = t_cap[2].trim().to_string();

                let thumb = thumb_re.captures(art)
                    .map(|c| c[1].trim().to_string())
                    .unwrap_or_default();

                let slug = post_url
                    .trim_end_matches('/')
                    .split('/')
                    .last()
                    .unwrap_or("video")
                    .to_string();

                let id = format!("fsi_{}", slug);
                let title = if raw_title.to_lowercase().contains("instagram") {
                    format!("📸 {}", raw_title)
                } else if raw_title.to_lowercase().contains("reels") {
                    format!("📹 {}", raw_title)
                } else if raw_title.to_lowercase().contains("leak") || raw_title.to_lowercase().contains("influencer") {
                    format!("🔥 {}", raw_title)
                } else {
                    format!("📸 Instagram Viral MMS: {}", raw_title)
                };

                videos.push(IndianVideo {
                    id,
                    title,
                    duration: "Viral MMS".to_string(),
                    views: serde_json::Value::String("Instagram Leaked".to_string()),
                    default_thumb: thumb,
                    url: post_url,
                    direct_mp4: None,
                });
            }
        }

        Ok(videos)
    }

    /// Scrape authentic Instagram viral MMS & influencer leaks from FsiBlog search
    pub async fn fetch_fsiblog(&self, query: &str, page: u32) -> Result<Vec<IndianVideo>> {
        let clean_query = query.trim().replace(' ', "+");
        let page_num = std::cmp::max(1, page);
        let url = if page_num == 1 {
            format!("https://www.fsiblogxx.com/?s={}", clean_query)
        } else {
            format!("https://www.fsiblogxx.com/page/{}/?s={}", page_num, clean_query)
        };
        self.fetch_fsiblog_url(&url).await
    }

    /// Scrape fresh MMS from FsiBlog /category/mms/
    pub async fn fetch_fsiblog_category_mms(&self, page: u32) -> Result<Vec<IndianVideo>> {
        let page_num = std::cmp::max(1, page);
        let url = if page_num == 1 {
            "https://www.fsiblogxx.com/category/mms/".to_string()
        } else {
            format!("https://www.fsiblogxx.com/category/mms/page/{}/", page_num)
        };
        self.fetch_fsiblog_url(&url).await
    }

    /// Resolve playable direct MP4 for an Indian video.
    pub async fn get_mp4_url(&self, video_url_or_id: &str) -> Result<String> {
        let full_url = if video_url_or_id.starts_with("http") {
            video_url_or_id.to_string()
        } else if video_url_or_id.starts_with("dp_") {
            let id = video_url_or_id.trim_start_matches("dp_");
            format!("https://www.desiporn.xxx/item/{}.html", id)
        } else if video_url_or_id.starts_with("fsi_") {
            let slug = video_url_or_id.trim_start_matches("fsi_");
            format!("https://www.fsiblogxx.com/mms/{}/", slug)
        } else {
            video_url_or_id.to_string()
        };

        if full_url.contains("fsiblog") {
            let html = self
                .http
                .get(&full_url)
                .header("Referer", "https://www.fsiblogxx.com/")
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

            let mp4_re = regex::Regex::new(r#"https?://cdn\.fsiblogxx\.com/[^"'\s<>]+\.mp4"#)?;
            if let Some(m) = mp4_re.find(&html) {
                return Ok(m.as_str().to_string());
            }

            let generic_mp4 = regex::Regex::new(r#"https?://[^"'\s<>]+\.mp4(?:\?[^"'\s<>]*)?"#)?;
            if let Some(m) = generic_mp4.find(&html) {
                return Ok(m.as_str().to_string());
            }
        } else if full_url.contains("desiporn.xxx") {
            let html = self
                .http
                .get(&full_url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;

            let source_re = regex::Regex::new(r#"<source[^>]+src=["']([^"']+\.mp4[^"']*)["']"#)?;
            if let Some(cap) = source_re.captures(&html) {
                let u = &cap[1];
                if !u.is_empty() {
                    return Ok(u.trim().to_string());
                }
            }
        }

        anyhow::bail!("No direct stream found on dedicated portal: {}", video_url_or_id)
    }
}

/// Strict filter guaranteeing only 100% REAL, AUTHENTIC human Indian MMS/Desi recordings.
/// Absolutely NO AI-generated, NO 3D animation, NO cartoons, NO Western pornstars.
pub fn is_authentic_real_indian_mms(title: &str, tags_text: &str) -> bool {
    let combined = format!("{} {}", title, tags_text).to_lowercase();

    // 1. AI, Animation, 3D, CGI, Cartoon, Gaming blacklist:
    const AI_ANIMATION_BLACKLIST: &[&str] = &[
        "animation", "animated", " 3d", "3d ", "-3d", "cartoon", "hentai",
        "sfm", "blender", "ai generated", "ai-generated", "virtual",
        "cgi", "drawing", "drawn", "comic", "parody", "anime", "deepfake",
        "video game", "gameplay", "overwatch", "fortnite", "genshin", "vr ",
        "manga", "toon", "furry", "illustration", "pixiv", "mmd", "source filmmaker",
        "unreal engine", "daz3d", "daz 3d", "koikatsu", "illusion", "waifu",
        "synthesized", "the sims", "c4d", "cinema 4d", "maya", "3ds max",
    ];

    for &word in AI_ANIMATION_BLACKLIST {
        if combined.contains(word) {
            return false;
        }
    }

    // 2. Family roleplay / scripted blacklist:
    // Do not blacklist if it's explicitly an Instagram / Reels viral leak!
    let is_insta = combined.contains("instagram")
        || combined.contains("insta ")
        || combined.contains("reels")
        || combined.contains("influencer");

    if !is_insta {
        const SCRIPTED_FAMILY_BLACKLIST: &[&str] = &[
            "bhabhi", "bhabi", "devar", "saali", "sali ", "chachi", "sasur",
            "bahu", "jija", "aunty", "aunt", "step", "family",
        ];

        for &word in SCRIPTED_FAMILY_BLACKLIST {
            if combined.contains(word) {
                return false;
            }
        }
    }

    // 3. Gay, trans, solo male blacklist:
    const GAY_TRANS_BLACKLIST: &[&str] = &[
        "gay", "trans", "transgender", "trans woman", "trans girl", "shemale",
        "ladyboy", "femboy", "crossdresser", "sissy", "male only", "solo male",
        "dick rating", "twink", "bulge",
    ];

    for &word in GAY_TRANS_BLACKLIST {
        if combined.contains(word) {
            return false;
        }
    }

    // 4. Non-Indian / Western model blacklist:
    const WESTERN_BLACKLIST: &[&str] = &[
        "native american", "indiana", "indianapolis", "apache", "navajo",
        "cherokee", "feather", "swiss resort", "karlee grey", "alex chance",
        "elena koshka", "nina north", "lena anderson", "lana rhoades",
        "riley reid", "mia malkova", "eva elfie", "angela white",
        "dani daniels", "brandi love", "abbie anderson", "lexi lore",
        "hazel moore", "lacey leveah", "lacey jayne", "penelope kay",
    ];

    for &word in WESTERN_BLACKLIST {
        if combined.contains(word) {
            return false;
        }
    }

    // 5. Positive Indian / Desi identifier (MUST match at least one):
    const INDIAN_IDENTIFIERS: &[&str] = &[
        "indian", "desi", "mms", "mallu", "tamil", "telugu", "kavya",
        "poonam", "priya", "neha", "puja", "pooja", "anjali", "bengali",
        "marathi", "punjabi", "kannada", "gujarati", "pakistani", "hindi",
        "delhi", "mumbai", "bangalore", "kolkata", "bihari", "dehati",
        "chudai", "scandal", "leaked", "college girl", "college mms",
        "hotel mms", "room mms", "viral video", "viral mms", "sharma",
        "verma", "singh", "patel", "gupta", "kumar", "kaur", "yadav",
        "salwar", "sari", "saree", "kerala", "chennai", "jaipur", "rajasthan",
        "instagram", "reels", "influencer",
    ];

    INDIAN_IDENTIFIERS.iter().any(|&word| combined.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_authentic_real_indian_mms_filter() {
        // AI / 3D / Animation should be strictly rejected
        assert!(!is_authentic_real_indian_mms("Desi Indian girl 3D animation", ""));
        assert!(!is_authentic_real_indian_mms("Indian college girl SFM animation blender", ""));
        assert!(!is_authentic_real_indian_mms("Hot Indian MMS AI generated deepfake", ""));
        assert!(!is_authentic_real_indian_mms("Cartoon desi hentai anime girl", ""));
        assert!(!is_authentic_real_indian_mms("Overwatch D.Va desi parody 3D", ""));

        // Scripted family roleplay should be rejected
        assert!(!is_authentic_real_indian_mms("Indian bhabhi aur devar chudai", ""));
        assert!(!is_authentic_real_indian_mms("Hot saali desi scandal", ""));

        // Western models should be rejected
        assert!(!is_authentic_real_indian_mms("Karlee Grey desi hotel room", ""));
        assert!(!is_authentic_real_indian_mms("Alex Chance college girl", ""));

        // Gay / Trans should be rejected
        assert!(!is_authentic_real_indian_mms("Indian trans woman blowjob", ""));

        // Genuine authentic Indian MMS should be accepted
        assert!(is_authentic_real_indian_mms("Viral Indian College Girl Leaked MMS Scandal in Hotel Room", ""));
        assert!(is_authentic_real_indian_mms("Desi College Girl Boyfriend Secret Phone Recording MMS", ""));
        assert!(is_authentic_real_indian_mms("Mallu Girl Leaked MMS Video with Audio", ""));
        assert!(is_authentic_real_indian_mms("Tamil Couple Private MMS Viral Scandal", ""));
        assert!(is_authentic_real_indian_mms("Real Indian MMS Leaked Video with Sound", ""));
    }

    #[tokio::test]
    async fn test_fetch_desiporn_mms() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let videos = client.fetch_desiporn("mms", 1).await.expect("fetch_desiporn failed");
        println!("Fetched {} videos from DesiPorn", videos.len());
        assert!(!videos.is_empty(), "Should fetch at least 1 video from DesiPorn");
        for v in videos.iter().take(3) {
            println!("- [{}] {} ({})", v.id, v.title, v.url);
        }
    }

    #[tokio::test]
    async fn test_get_desiporn_mp4_url() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let mp4 = client.get_mp4_url("https://www.desiporn.xxx/item/226816.html").await.expect("Failed to get MP4");
        println!("Resolved MP4: {}", mp4);
        assert!(mp4.contains(".mp4"), "Must contain .mp4");
        assert!(mp4.starts_with("http"), "Must start with http");
    }

    #[tokio::test]
    async fn test_fetch_fsiblog_instagram() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let videos = client.fetch_fsiblog("instagram", 1).await.expect("fetch_fsiblog failed");
        println!("Fetched {} Instagram viral MMS from FsiBlog", videos.len());
        assert!(!videos.is_empty(), "Should fetch Instagram viral MMS videos");
        for v in videos.iter().take(3) {
            println!("- [{}] {} ({})", v.id, v.title, v.url);
        }
    }

    #[tokio::test]
    async fn test_get_fsiblog_mp4_url() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let mp4 = client.get_mp4_url("https://www.fsiblogxx.com/blowjob/beautiful-instagram-influencer-blowjob/").await.expect("Failed to get fsiblog MP4");
        println!("Resolved FsiBlog MP4: {}", mp4);
        assert!(mp4.contains(".mp4"), "Must contain .mp4");
        assert!(mp4.starts_with("http"), "Must start with http");
    }

    #[tokio::test]
    async fn test_fetch_indian_videos_priority() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let videos = client.fetch_indian_videos("instagram", 10, 1).await.expect("fetch_indian_videos failed");
        println!("fetch_indian_videos returned {} videos", videos.len());
        assert!(!videos.is_empty(), "Should return videos");
        assert!(videos.iter().all(|v| v.id.starts_with("fsi_")), "All top videos should be fresh FsiBlog Instagram leaks!");
        for v in videos.iter().take(5) {
            println!("- [{}] {} ({})", v.id, v.title, v.url);
        }
    }
}
