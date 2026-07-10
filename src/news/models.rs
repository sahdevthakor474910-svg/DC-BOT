#[derive(Debug, Clone)]
pub struct NewsArticle {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub source: String,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}
