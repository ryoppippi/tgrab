use anyhow::Result;
use clap::Parser;
use tgrab::{
    bluesky, create_client,
    router::{Service, route},
    twitter, youtube,
};

/// Trailing `--help` content. Lives in the binary rather than an external doc so
/// that agents discovering tgrab at runtime get the full contract from `--help`
/// alone, and so it sits next to the patterns in `router.rs` that define it.
const AFTER_HELP: &str = "\
Supported URL patterns:
  YouTube      youtube.com/watch?v=, youtu.be/, youtube.com/embed/,
               youtube.com/v/, m.youtube.com/watch?v=
  Twitter / X  x.com/<user>/status/<id>, twitter.com/<user>/status/<id>
  Bluesky      bsky.app/profile/<handle-or-did>/post/<id>

A `www.` prefix is accepted, and the scheme may be omitted.

Output format:
  YouTube      `# <title>`, then one line per caption as `[M:SS] text`
  Twitter / X  `@username:`, a blank line, then the post text
  Bluesky      `@handle on Bluesky:`, a blank line, then the post text

Examples:
  tgrab https://www.youtube.com/watch?v=dQw4w9WgXcQ
  tgrab --lang ja https://www.youtube.com/watch?v=dQw4w9WgXcQ
  tgrab https://x.com/user/status/1234567890
  tgrab https://bsky.app/profile/user.bsky.social/post/abc123

Notes for agents:
  Transcripts can be long — run tgrab in a subagent to keep the main context
  clean. For pages needing JavaScript rendering or a login, tgrab cannot help;
  use agent-browser (https://github.com/vercel-labs/agent-browser) instead.";

#[derive(Parser)]
#[command(
    name = "tgrab",
    about = "Fetch text content from YouTube, Twitter/X, and Bluesky",
    long_about = "Fetch text content from services that normally require a browser or a login: \
                  YouTube transcripts, Twitter/X posts, and Bluesky posts. The service is \
                  detected automatically from the URL.",
    after_help = AFTER_HELP
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
