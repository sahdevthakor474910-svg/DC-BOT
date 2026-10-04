import re

with open('src/reddit/client.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add MEMESGUY_INDEX
content = re.sub(r'pub struct RedditClient',
                 'static MEMESGUY_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);\n\npub struct RedditClient',
                 content)

# Update fetch_memesguy_memes
memesguy_old = r'''    pub async fn fetch_memesguy_memes\(&self\) -> Result<Vec<MemeGuyPost>> \{
        debug!\("Fetching memes from memesguy\.com"\);
        let html = self\.client\.get\("https://memesguy\.com/"\)
            \.send\(\)
            \.await\?
            \.text\(\)
            \.await\?;'''
            
memesguy_new = '''    pub async fn fetch_memesguy_memes(&self) -> Result<Vec<MemeGuyPost>> {
        let urls = [
            "https://memesguy.com/",
            "https://memesguy.com/timeline",
            "https://memesguy.com/year/2026",
            "https://memesguy.com/year/2025",
            "https://memesguy.com/year/2024",
        ];
        let idx = MEMESGUY_INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % urls.len();
        let target_url = urls[idx];
        debug!("Fetching memes from {}", target_url);
        let html = self.client.get(target_url)
            .send()
            .await?
            .text()
            .await?;'''
content = re.sub(memesguy_old, memesguy_new, content)

# Add iterators helper
iter_helper = '''
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub fn iterators() -> &'static Mutex<HashMap<String, String>> {
    static ITERS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    ITERS.get_or_init(|| Mutex::new(HashMap::new()))
}
'''
content = content + iter_helper

# Update fetch_from_scrolller
scrolller_old = r'''    async fn fetch_from_scrolller\(&self, subreddit: &str, limit: u32\) -> Result<Vec<RedditPost>> \{
        let query = r#"
            query SubredditQuery\(\$url: String!, \$limit: Int!\) \{
              getSubreddit\(data: \{ url: \$url, limit: \$limit \}\) \{
                id
                url
                title
                children \{
                  items \{'''
scrolller_new = '''    async fn fetch_from_scrolller(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        let query = r#"
            query SubredditQuery($url: String!, $limit: Int!, $iterator: String) {
              getSubreddit(data: { url: $url, limit: $limit, iterator: $iterator }) {
                id
                url
                title
                children {
                  iterator
                  items {'''
content = re.sub(scrolller_old, scrolller_new, content)

vars_old = r'''        let variables = serde_json::json!\(\{
            "url": format!\("/r/\{\}", subreddit\),
            "limit": limit
        \}\);'''
vars_new = '''        let iter_key = format!("/r/{}", subreddit);
        let iterator = {
            let iters = crate::reddit::client::iterators().lock().unwrap();
            iters.get(&iter_key).cloned()
        };
        let mut variables = serde_json::json!({
            "url": iter_key.clone(),
            "limit": limit
        });
        if let Some(it) = iterator {
            variables.as_object_mut().unwrap().insert("iterator".to_string(), serde_json::json!(it));
        }'''
content = re.sub(vars_old, vars_new, content)

response_old = r'''        let subreddit_data = match response\.data\.get_subreddit \{
            Some\(data\) => data,
            None => \{
                warn!\("Subreddit r/\{\} not found on Scrolller", subreddit\);
                return Ok\(vec!\[\]\);
            \}
        \};

        let mut posts = Vec::new\(\);'''
response_new = '''        let subreddit_data = match response.data.get_subreddit {
            Some(data) => data,
            None => {
                warn!("Subreddit r/{} not found on Scrolller", subreddit);
                return Ok(vec![]);
            }
        };

        if let Some(it) = &subreddit_data.children.iterator {
            crate::reddit::client::iterators().lock().unwrap().insert(iter_key, it.clone());
        }

        let mut posts = Vec::new();'''
content = re.sub(response_old, response_new, content)

struct_old = r'''struct ScrolllerChildren \{
    items: Vec<ScrolllerItem>,'''
struct_new = '''struct ScrolllerChildren {
    iterator: Option<String>,
    items: Vec<ScrolllerItem>,'''
content = re.sub(struct_old, struct_new, content)

with open('src/reddit/client.rs', 'w', encoding='utf-8') as f:
    f.write(content)
