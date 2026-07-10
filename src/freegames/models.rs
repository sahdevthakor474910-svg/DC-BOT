/// A free-game or giveaway item, unified across all stores.
#[derive(Debug, Clone)]
pub struct FreeGame {
    /// Stable deduplication ID (store::game_id or store::slug).
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Human-readable original price, e.g. "$19.99".
    pub original_price: Option<String>,
    /// Store name: "Epic Games", "Steam", etc.
    pub store: String,
    /// Direct URL to the store listing.
    pub url: String,
    /// CDN thumbnail / cover art URL.
    pub thumbnail_url: Option<String>,
    /// When the giveaway / free period ends.
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Short instructions shown in the embed footer.
    pub claim_instructions: String,
}
