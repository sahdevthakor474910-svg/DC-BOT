use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use super::models::Tweet;

/// The two X accounts to monitor — hardcoded as requested.
pub const ACCOUNTS: &[(&str, &str)] = &[
    ("dmc_poc",    "🌍 DMC Global"),
    ("dmc_poc_jp", "🌏 DMC Asia/JP"),
];

// ─────────────────────────────────────────────────────────────────────────────
// X / Twitter GraphQL + guest token constants
// ─────────────────────────────────────────────────────────────────────────────

const GUEST_TOKEN_URL: &str = "https://api.twitter.com/1.1/guest/activate.json";
const BEARER: &str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs=1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

/// Serde shapes for X's guest-token and GraphQL responses.
#[derive(Deserialize)]
struct GuestToken {
    guest_token: String,
}

#[derive(Deserialize)]
struct TimelineResponse {
    data: TimelineData,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TimelineData {
    user: UserResult,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct UserResult {
    result: UserResultInner,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct UserResultInner {
    timeline_v2: TimelineV2,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TimelineV2 {
    timeline: Timeline,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Timeline {
    instructions: Vec<Instruction>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
enum Instruction {
    TimelineAddEntries { entries: Vec<Entry> },
    #[serde(other)]
    Other,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Entry {
    content: EntryContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntryContent {
    item_content: Option<ItemContent>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ItemContent {
    tweet_results: Option<TweetResults>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TweetResults {
    result: Option<TweetResult>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TweetResult {
    rest_id: Option<String>,
    legacy: Option<TweetLegacy>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
struct TweetLegacy {
    full_text: Option<String>,
    created_at: Option<String>,
    id_str: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public client
// ─────────────────────────────────────────────────────────────────────────────

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

    /// Fetch the latest tweets for `username`. Uses X guest token flow.
    pub async fn fetch_tweets(&self, username: &str, limit: usize) -> Result<Vec<Tweet>> {
        // ── Step 1: Obtain a guest token ─────────────────────────────────────
        let guest_token = self.get_guest_token().await?;
        info!("🐦 Got guest token for @{username}");

        // ── Step 2: Resolve username → user ID ──────────────────────────────
        let user_id = self.resolve_user_id(username, &guest_token).await?;
        info!("🐦 Resolved @{username} → user_id={user_id}");

        // ── Step 3: Fetch timeline ───────────────────────────────────────────
        self.fetch_timeline(&user_id, username, &guest_token, limit)
            .await
    }

    // ── Private ──────────────────────────────────────────────────────────────

    async fn get_guest_token(&self) -> Result<String> {
        let resp = self
            .http
            .post(GUEST_TOKEN_URL)
            .header("Authorization", format!("Bearer {}", BEARER))
            .send()
            .await?
            .error_for_status()?
            .json::<GuestToken>()
            .await?;
        Ok(resp.guest_token)
    }

    async fn resolve_user_id(&self, username: &str, guest_token: &str) -> Result<String> {
        let url = format!(
            "https://api.twitter.com/graphql/NimuplG1OB7Fd2btCLdBOw/UserByScreenName\
?variables=%7B%22screen_name%22%3A%22{}%22%2C%22withSafetyModeUserFields%22%3Atrue%7D\
&features=%7B%22hidden_profile_subscriptions_enabled%22%3Atrue%2C%22rweb_tipjar_consumption_enabled%22%3Atrue%2C%22responsive_web_graphql_exclude_directive_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22subscriptions_verification_info_is_identity_verified_enabled%22%3Atrue%2C%22subscriptions_verification_info_verified_since_enabled%22%3Atrue%2C%22highlights_tweets_tab_ui_enabled%22%3Atrue%2C%22responsive_web_twitter_article_notes_tab_enabled%22%3Atrue%2C%22creator_subscriptions_tweet_preview_api_enabled%22%3Atrue%2C%22hidden_profile_likes_enabled%22%3Atrue%2C%22subscriptions_feature_can_gift_premium%22%3Afalse%7D\
&fieldToggles=%7B%22withAffiliatesHighlights%22%3Afalse%7D",
            username
        );

        #[derive(Deserialize)]
        struct UserByScreenName {
            data: UserByScreenNameData,
        }
        #[derive(Deserialize)]
        struct UserByScreenNameData {
            user: UserByScreenNameUser,
        }
        #[derive(Deserialize)]
        struct UserByScreenNameUser {
            result: UserByScreenNameResult,
        }
        #[derive(Deserialize)]
        struct UserByScreenNameResult {
            rest_id: String,
        }

        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", BEARER))
            .header("x-guest-token", guest_token)
            .header("x-twitter-active-user", "yes")
            .header("x-twitter-client-language", "en")
            .send()
            .await?
            .error_for_status()?
            .json::<UserByScreenName>()
            .await?;

        Ok(resp.data.user.result.rest_id)
    }

    async fn fetch_timeline(
        &self,
        user_id: &str,
        username: &str,
        guest_token: &str,
        limit: usize,
    ) -> Result<Vec<Tweet>> {
        let count = (limit * 2).max(20); // ask for more in case some are pinned/ads
        let url = format!(
            "https://api.twitter.com/graphql/V7H0Ap3_Hh2FyS75OCDO3Q/UserTweets\
?variables=%7B%22userId%22%3A%22{}%22%2C%22count%22%3A{}%2C%22includePromotedContent%22%3Afalse%2C%22withQuickPromoteEligibilityTweetFields%22%3Atrue%2C%22withVoice%22%3Atrue%2C%22withV2Timeline%22%3Atrue%7D\
&features=%7B%22rweb_lists_timeline_redesign_enabled%22%3Atrue%2C%22responsive_web_graphql_exclude_directive_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22creator_subscriptions_tweet_preview_api_enabled%22%3Atrue%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22responsive_web_graphql_skip_user_profile_image_extensions_enabled%22%3Afalse%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22view_counts_everywhere_api_enabled%22%3Atrue%2C%22longform_notetweets_consumption_enabled%22%3Atrue%2C%22tweet_awards_web_tipping_enabled%22%3Afalse%2C%22freedom_of_speech_not_reach_fetch_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22longform_notetweets_rich_text_read_enabled%22%3Atrue%2C%22longform_notetweets_inline_media_enabled%22%3Atrue%2C%22responsive_web_media_download_video_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Afalse%7D\
&fieldToggles=%7B%22withArticleRichContentState%22%3Afalse%7D",
            user_id, count
        );

        let raw: serde_json::Value = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", BEARER))
            .header("x-guest-token", guest_token)
            .header("x-twitter-active-user", "yes")
            .header("x-twitter-client-language", "en")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // Walk the JSON tree manually so we can tolerate schema drift
        let entries = raw
            .pointer("/data/user/result/timeline_v2/timeline/instructions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|inst| {
                        if inst.get("type")?.as_str()? == "TimelineAddEntries" {
                            inst.get("entries")?.as_array().cloned()
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut tweets = Vec::new();

        for entry in entries.iter().take(limit * 3) {
            // Skip cursor entries
            let entry_id = entry.pointer("/entryId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if entry_id.starts_with("cursor-") {
                continue;
            }

            let legacy = match entry.pointer("/content/itemContent/tweet_results/result/legacy") {
                Some(v) => v,
                None => continue,
            };

            let tweet_id = legacy.get("id_str")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if tweet_id.is_empty() {
                continue;
            }

            let text = legacy.get("full_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let pub_date = legacy.get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let link = format!("https://twitter.com/{}/status/{}", username, tweet_id);

            tweets.push(Tweet {
                id: tweet_id,
                account: username.to_string(),
                text,
                link,
                pub_date,
                translated_text: None,
            });

            if tweets.len() >= limit {
                break;
            }
        }

        if tweets.is_empty() {
            warn!("🐦 No tweets extracted from timeline response for @{}", username);
        }

        Ok(tweets)
    }
}
