use agent_fetcher::{
    bluesky, create_client,
    router::{Service, route},
    twitter, youtube,
};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "agent-fetcher",
    about = "Fetch text content from YouTube, Twitter/X, and Bluesky"
)]
struct Cli {
    /// URL to fetch (YouTube, Twitter/X, or Bluesky)
    url: String,

    /// Language code for YouTube transcripts, e.g. "en" or "ja"
    #[arg(short, long)]
    lang: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = create_client()?;

    match route(&cli.url)? {
        Service::YouTube(video_id) => {
            let transcript =
                youtube::fetch_transcript(&client, &video_id, cli.lang.as_deref()).await?;

            println!("# {}", transcript.title);
            println!();
            for line in &transcript.lines {
                let secs = line.offset as u64;
                println!("[{}:{:02}] {}", secs / 60, secs % 60, line.text);
            }
        }
        Service::Twitter(url) => {
            let content = twitter::fetch_post(&client, &url).await?;
            println!("{content}");
        }
        Service::Bluesky(url) => {
            let content = bluesky::fetch_post(&client, &url).await?;
            println!("{content}");
        }
    }

    Ok(())
}
