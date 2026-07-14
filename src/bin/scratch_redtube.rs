use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()?;

    // Search query Brazzers to get a Brazzers video from RedTube
    // RedTube API or simple query
    let search_url = "https://api.redtube.com/?data=redtube.Videos.searchVideos&output=json&search=brazzers&count=1";
    let resp = client.get(search_url).send().await?.text().await?;
    println!("API Response: {}", resp);

    // Parse the first video url
    let re = regex::Regex::new(r#""url"\s*:\s*"([^"]+)""#)?;
    if let Some(cap) = re.captures(&resp) {
        let video_url = cap[1].replace("\\/", "/");
        println!("Found video URL: {}", video_url);

        // Fetch video page HTML from RedTube
        let video_html = client.get(&video_url).send().await?.text().await?;
        std::fs::write("redtube_page.html", &video_html)?;
        println!("Saved RedTube page HTML to redtube_page.html (length: {})", video_html.len());

        // Search for possible stream/MP4 links
        let mp4_re = regex::Regex::new(r#"https://[^"']+\.mp4[^"']*"#)?;
        println!("Checking for direct .mp4 links in RedTube page...");
        for mat in mp4_re.find_iter(&video_html) {
            println!("  Found .mp4 link candidate: {}", mat.as_str());
        }

        // Search for video tags with source
        let source_re = regex::Regex::new(r#"<source[^>]+src=["']([^"']+)["']"#)?;
        for cap in source_re.captures_iter(&video_html) {
            println!("  Found <source> tag: {}", &cap[1]);
        }

        // Search for video Url configurations inside JSON
        let video_url_re = regex::Regex::new(r#""videoUrl"\s*:\s*"([^"]+)""#)?;
        for cap in video_url_re.captures_iter(&video_html) {
            println!("  Found videoUrl json: {}", &cap[1]);
        }
    } else {
        println!("Could not find any video link in RedTube API response!");
    }

    Ok(())
}
