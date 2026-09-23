#[derive(Debug, Clone)]
pub struct XnxxVideo {
    /// Unique ID extracted from the URL path
    pub video_id: String,
    /// Human-readable title
    pub title: String,
    /// Full page URL
    pub url: String,
    /// CDN thumbnail URL
    pub thumbnail: String,
    /// Duration string, e.g. "17:06"
    pub duration: String,
    /// View count string, e.g. "9.2M"
    pub views: String,
}
