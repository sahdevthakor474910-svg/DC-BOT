#[derive(Debug, Clone)]
pub struct OkXxxVideo {
    /// Unique ID extracted from the URL path (e.g. "758382")
    pub video_id: String,
    /// Human-readable title
    pub title: String,
    /// Full page URL (e.g. https://ok.xxx/video/758382/)
    pub url: String,
    /// CDN thumbnail URL (640×360)
    pub thumbnail: String,
    /// Duration string, e.g. "17:06"
    pub duration: String,
    /// View count string, e.g. "9.2K"
    pub views: String,
}
