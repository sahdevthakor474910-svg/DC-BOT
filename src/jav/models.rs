/// A single JAV title fetched from the R18.dev API.
#[derive(Debug, Clone)]
pub struct JavTitle {
    /// Unique content ID (e.g. "abc00123")
    pub content_id: String,
    /// Movie title
    pub title: String,
    /// Direct link to the R18.dev product page
    pub url: String,
    /// Cover jacket image URL
    pub cover_url: Option<String>,
    /// Primary actress name(s)
    pub actresses: Vec<String>,
    /// Studio / maker name
    pub studio: Option<String>,
    /// Release date string (e.g. "2024-07-10")
    pub release_date: Option<String>,
    /// Whether this came from the "popular" endpoint
    pub is_popular: bool,
}
