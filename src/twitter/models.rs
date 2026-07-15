/// A single tweet fetched from a Nitter RSS feed.
#[derive(Debug, Clone)]
pub struct Tweet {
    /// Unique tweet ID (numeric part extracted from the Nitter/X URL).
    pub id: String,
    /// X username that posted this tweet, e.g. "dmc_poc".
    pub account: String,
    /// Plain-text body of the tweet (HTML stripped).
    pub text: String,
    /// Canonical Twitter/X URL so users can open the original tweet.
    pub link: String,
    /// Human-readable publish date/time string (may be empty).
    pub pub_date: String,
    /// Cached English translation (if the original text was Japanese).
    pub translated_text: Option<String>,
}
