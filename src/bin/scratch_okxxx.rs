use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()?;

    // Fetch homepage to get a video URL
    let home_html = client.get("https://ok.xxx/").send().await?.text().await?;
    println!("Fetched home page (length {})", home_html.len());
    
    // Find the first video page link
    // Look for "/video/..."
    let re = regex::Regex::new(r#"/video/\d+/"#)?;
    if let Some(mat) = re.find(&home_html) {
        let video_path = mat.as_str();
        let video_url = format!("https://ok.xxx{}", video_path);
        println!("Found video URL: {}", video_url);

        // Fetch video page HTML
        let video_html = client.get(&video_url).send().await?.text().await?;
        std::fs::write("video_page.html", &video_html)?;
        println!("Saved video page HTML to video_page.html (length: {})", video_html.len());

        // Search for possible stream/MP4 links
        let mp4_re = regex::Regex::new(r#"https://[^"']+\.mp4[^"']*"#)?;
        println!("Checking for direct .mp4 links in video page...");
        for mat in mp4_re.find_iter(&video_html) {
            println!("  Found .mp4 link candidate: {}", mat.as_str());
        }

        // Also search for source tags
        let source_re = regex::Regex::new(r#"<source[^>]+src=["']([^"']+)["']"#)?;
        for cap in source_re.captures_iter(&video_html) {
            println!("  Found <source> tag: {}", &cap[1]);
        }

        // Search for JWPlayer/player configs
        let file_re = regex::Regex::new(r#"file\s*:\s*["']([^"']+)["']"#)?;
        for cap in file_re.captures_iter(&video_html) {
            println!("  Found player 'file' config: {}", &cap[1]);
        }
    } else {
        println!("Could not find any video link on homepage!");
    }

    Ok(())
}
