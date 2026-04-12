use anyhow::{Result, anyhow};
use regex::Regex;
use std::sync::LazyLock;

use crate::HttpClient;

static RE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?bsky\.app/profile/([^/]+)/post/([A-Za-z0-9]+)")
        .expect("invalid Bluesky URL regex")
});

/// Extract (handle, rkey) from a Bluesky post URL.
///
/// # Examples
///
/// ```
/// use agent_fetcher::bluesky::parse_url;
/// assert_eq!(
///     parse_url("https://bsky.app/profile/user.bsky.social/post/abc123"),
///     Some(("user.bsky.social".into(), "abc123".into()))
/// );
/// ```
pub fn parse_url(url: &str) -> Option<(String, String)> {
    RE_URL
        .captures(url)
        .map(|c| (c[1].to_string(), c[2].to_string()))
}

/// Fetch a Bluesky post via the AT Protocol public API and return formatted text.
///
/// Flow:
/// 1. Resolve the handle to a DID via `com.atproto.identity.resolveHandle`
/// 2. Fetch the post thread via `app.bsky.feed.getPostThread`
pub async fn fetch_post(client: &HttpClient, url: &str) -> Result<String> {
    let (handle, rkey) =
        parse_url(url).ok_or_else(|| anyhow!("Could not parse Bluesky URL: {url}"))?;

    // Resolve handle → DID
    let resolve_url = format!(
        "https://public.api.bsky.app/xrpc/com.atproto.identity.resolveHandle?handle={handle}"
    );
    let resolve_resp = client
        .get(resolve_url.clone(), None, None)
        .await
        .map_err(|e| anyhow!("HTTP error resolving handle: {e}"))?;
    let resolve_body = resolve_resp
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read resolve response: {e}"))?;
    let resolve_json: serde_json::Value = serde_json::from_str(&resolve_body)?;
    let did = resolve_json["did"]
        .as_str()
        .ok_or_else(|| anyhow!("Could not resolve handle '{handle}' to a DID"))?;

    // Fetch post thread
    let at_uri = format!("at://{did}/app.bsky.feed.post/{rkey}");
    let thread_url =
        format!("https://public.api.bsky.app/xrpc/app.bsky.feed.getPostThread?uri={at_uri}");
    let thread_resp = client
        .get(thread_url.clone(), None, None)
        .await
        .map_err(|e| anyhow!("HTTP error fetching post thread: {e}"))?;
    let thread_body = thread_resp
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read thread response: {e}"))?;
    let thread_json: serde_json::Value = serde_json::from_str(&thread_body)?;

    let post = &thread_json["thread"]["post"];
    let text = post["record"]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("No text in post record"))?;
    let author = post["author"]["handle"].as_str().unwrap_or(&handle);

    Ok(format!("@{author} on Bluesky:\n\n{text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_handle_and_rkey() {
        assert_eq!(
            parse_url("https://bsky.app/profile/user.bsky.social/post/abc123"),
            Some(("user.bsky.social".into(), "abc123".into()))
        );
    }

    #[test]
    fn parse_url_did_profile() {
        assert_eq!(
            parse_url("https://bsky.app/profile/did:plc:abc123/post/xyz789"),
            Some(("did:plc:abc123".into(), "xyz789".into()))
        );
    }

    #[test]
    fn parse_url_none_for_non_post() {
        assert_eq!(parse_url("https://bsky.app/profile/user.bsky.social"), None);
    }
}
