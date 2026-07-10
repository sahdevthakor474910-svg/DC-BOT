use serde::Deserialize;

// ────────────────────────────────────────────────────────────────────────────
// Top-level response
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RedditResponse {
    pub data: ListingData,
}

#[derive(Debug, Deserialize)]
pub struct ListingData {
    pub children: Vec<PostWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct PostWrapper {
    pub data: RedditPost,
}

// ────────────────────────────────────────────────────────────────────────────
// Post fields
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct RedditPost {
    /// Unique base-36 post identifier.
    pub id: String,
    pub title: String,
    pub author: String,
    pub subreddit: String,
    pub score: i64,
    pub url: String,
    /// Alternate URL set by some link-type posts.
    pub url_overridden_by_dest: Option<String>,
    /// Hint provided by Reddit about the post type.
    pub post_hint: Option<String>,
    /// True when the post contains a Reddit-hosted video.
    pub is_video: bool,
    /// True when the post is marked NSFW.
    pub over_18: bool,
    /// True when the post is marked as a spoiler.
    #[serde(default)]
    pub spoiler: bool,
    /// True for pinned/announcement posts.
    pub stickied: bool,
    /// Relative URL: "/r/memes/comments/…"
    pub permalink: String,
    /// Populated for hosted:video / rich:video posts.
    pub media: Option<RedditMedia>,
    /// Reddit gallery / image previews.
    pub preview: Option<Preview>,
}


#[derive(Debug, Deserialize, Clone)]
pub struct RedditMedia {
    pub reddit_video: Option<RedditVideo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedditVideo {
    /// Direct MP4 URL without audio (but playable in Discord).
    pub fallback_url: String,
    pub is_gif: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Preview {
    pub images: Vec<PreviewImage>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PreviewImage {
    pub source: ImageSource,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImageSource {
    pub url: String,
}
