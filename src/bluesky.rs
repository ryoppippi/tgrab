use anyhow::{anyhow, Result};

use crate::{twitter::extract_content, HttpClient};

/// Rewrite a Bluesky URL to its FxEmbed proxy equivalent.
///
/// `bsky.app` → `fxbsky.app`
///
/// # Examples
///
/// ```
/// use agent_fetcher::bluesky::rewrite_url;
/// assert_eq!(
///     rewrite_url("https://bsky.app/profile/user.bsky.social/post/abc"),
///     "https://fxbsky.app/profile/user.bsky.social/post/abc"
/// );
/// ```
pub fn rewrite_url(url: &str) -> String {
    url.replace("://bsky.app/", "://fxbsky.app/")
}

/// Fetch a Bluesky post via the FxEmbed proxy and return its text content.
pub async fn fetch_post(client: &HttpClient, url: &str) -> Result<String> {
    let proxy_url = rewrite_url(url);

    let response = client
        .get(proxy_url.clone(), None, None)
        .await
        .map_err(|e| anyhow!("HTTP error fetching {proxy_url}: {e}"))?;

    let html = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response body: {e}"))?;

    extract_content(&html)
        .ok_or_else(|| anyhow!("Could not extract post content from FxEmbed response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── rewrite_url ───────────────────────────────────────────────────────────

    #[test]
    fn rewrites_bsky_app() {
        assert_eq!(
            rewrite_url("https://bsky.app/profile/user.bsky.social/post/abc123"),
            "https://fxbsky.app/profile/user.bsky.social/post/abc123"
        );
    }

    #[test]
    fn rewrites_did_profile() {
        assert_eq!(
            rewrite_url("https://bsky.app/profile/did:plc:abc123/post/xyz789"),
            "https://fxbsky.app/profile/did:plc:abc123/post/xyz789"
        );
    }

    #[test]
    fn preserves_full_path() {
        assert_eq!(
            rewrite_url("https://bsky.app/profile/handle.bsky.social/post/3kv7"),
            "https://fxbsky.app/profile/handle.bsky.social/post/3kv7"
        );
    }
}
