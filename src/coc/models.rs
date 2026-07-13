use chrono::{DateTime, Utc};

/// A single CoC update/event/reward post.
#[derive(Debug, Clone)]
pub struct CocUpdate {
    /// Unique dedup ID (e.g. "reddit::abc123" or "blog::https://...")
    pub id: String,
    /// Post/article title
    pub title: String,
    /// Short description or summary
    pub description: Option<String>,
    /// Direct URL to the post/article
    pub url: String,
    /// Thumbnail / banner image
    pub image_url: Option<String>,
    /// Source label shown in embed footer
    pub source: String,
    /// Publication time (used for sorting)
    pub published_at: Option<DateTime<Utc>>,
    /// Tag/category e.g. "Update", "Event", "Free Reward"
    pub tag: String,
}
