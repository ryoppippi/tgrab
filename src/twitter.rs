use anyhow::{Result, anyhow};
use regex::Regex;
use std::sync::LazyLock;

use crate::HttpClient;

static RE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:www\.)?(?:twitter\.com|x\.com)/(\w+)/status/(\d+)")
        .expect("invalid Twitter URL regex")
});

/// Build the fxtwitter JSON API URL from a Twitter / X post URL.
///
/// # Examples
///
/// ```
/// use tgrab::twitter::api_url;
/// assert_eq!(
///     api_url("https://x.com/user/status/123456"),
///     Some("https://api.fxtwitter.com/user/status/123456".into())
/// );
/// ```
pub fn api_url(url: &str) -> Option<String> {
    RE_URL
        .captures(url)
        .map(|caps| format!("https://api.fxtwitter.com/{}/status/{}", &caps[1], &caps[2]))
}

/// Fetch a Twitter / X post via the fxtwitter JSON API and return formatted text.
pub async fn fetch_post(client: &HttpClient, url: &str) -> Result<String> {
    let api = api_url(url).ok_or_else(|| anyhow!("Could not parse Twitter URL: {url}"))?;

    let response = client
        .get(api.clone(), None, None)
        .await
        .map_err(|e| anyhow!("HTTP error fetching {api}: {e}"))?;

    let body = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response body: {e}"))?;

    let json: serde_json::Value = serde_json::from_str(&body)?;

    let tweet = &json["tweet"];
    let text = tweet["text"]
        .as_str()
        .ok_or_else(|| anyhow!("No text field in fxtwitter response"))?;
    let author = tweet["author"]["screen_name"].as_str().unwrap_or("unknown");

    Ok(format!("@{author}:\n\n{text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_from_x_com() {
        assert_eq!(
            api_url("https://x.com/user/status/123456"),
            Some("https://api.fxtwitter.com/user/status/123456".into())
        );
    }

    #[test]
    fn api_url_from_twitter_com() {
        assert_eq!(
            api_url("https://twitter.com/user/status/123456"),
            Some("https://api.fxtwitter.com/user/status/123456".into())
        );
    }

    #[test]
    fn api_url_from_www_twitter_com() {
        assert_eq!(
            api_url("https://www.twitter.com/user/status/123456"),
            Some("https://api.fxtwitter.com/user/status/123456".into())
        );
    }

    #[test]
    fn api_url_preserves_user_and_id() {
        assert_eq!(
            api_url("https://x.com/rustlang/status/9999"),
            Some("https://api.fxtwitter.com/rustlang/status/9999".into())
        );
    }

    #[test]
    fn api_url_none_for_non_status_url() {
        assert_eq!(api_url("https://x.com/user"), None);
    }

    /// Run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn integration_fetch_post_x_com() {
        let client = crate::create_client().unwrap();
        let result = fetch_post(
            &client,
            "https://x.com/ryoppippi/status/1909369320078999673",
        )
        .await
        .unwrap();

        assert!(result.starts_with("@ryoppippi:"));
        assert!(result.contains("任意のサイトをMCP Serverに変えちゃうヤバいやつ作ったよ"));
        assert!(result.contains("sitemcp"));
    }
}
