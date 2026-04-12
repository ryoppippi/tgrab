use anyhow::{anyhow, Result};
use std::sync::LazyLock;

use regex::Regex;

use crate::HttpClient;

static RE_CONTENT: LazyLock<Regex> = LazyLock::new(|| {
    // Matches both attribute orderings: property first or content first
    Regex::new(
        r#"(?x)
        <meta[^>]*
        (?:
          property=["\']og:description["\'][^>]*content=["\']([^"\']*)["\']
          |
          content=["\']([^"\']*)["\'][^>]*property=["\']og:description["\']
        )
        [^>]*/?>
        "#,
    )
    .expect("invalid og:description regex")
});

/// Rewrite a Twitter / X URL to its FxEmbed proxy equivalent.
///
/// - `x.com` → `fixupx.com`
/// - `twitter.com` (with or without `www.`) → `fxtwitter.com`
///
/// # Examples
///
/// ```
/// use agent_fetcher::twitter::rewrite_url;
/// assert_eq!(
///     rewrite_url("https://x.com/user/status/123"),
///     "https://fixupx.com/user/status/123"
/// );
/// ```
pub fn rewrite_url(url: &str) -> String {
    url.replace("://x.com/", "://fixupx.com/")
        .replace("://www.twitter.com/", "://fxtwitter.com/")
        .replace("://twitter.com/", "://fxtwitter.com/")
}

/// Decode common HTML entities from an attribute value.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Extract the post text from an FxEmbed HTML page via the `og:description` meta tag.
pub fn extract_content(html: &str) -> Option<String> {
    RE_CONTENT
        .captures(html)
        .map(|caps| {
            // Group 1 = property-first; group 2 = content-first
            caps.get(1)
                .or_else(|| caps.get(2))
                .map(|m| decode_entities(m.as_str()))
        })
        .flatten()
}

/// Fetch a Twitter / X post via the FxEmbed proxy and return its text content.
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

    extract_content(&html).ok_or_else(|| anyhow!("Could not extract post content from FxEmbed response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── rewrite_url ───────────────────────────────────────────────────────────

    #[test]
    fn rewrites_x_com() {
        assert_eq!(
            rewrite_url("https://x.com/user/status/123456"),
            "https://fixupx.com/user/status/123456"
        );
    }

    #[test]
    fn rewrites_twitter_com() {
        assert_eq!(
            rewrite_url("https://twitter.com/user/status/123456"),
            "https://fxtwitter.com/user/status/123456"
        );
    }

    #[test]
    fn rewrites_www_twitter_com() {
        assert_eq!(
            rewrite_url("https://www.twitter.com/user/status/123456"),
            "https://fxtwitter.com/user/status/123456"
        );
    }

    #[test]
    fn preserves_path_and_query() {
        assert_eq!(
            rewrite_url("https://x.com/rustlang/status/9999?s=20"),
            "https://fixupx.com/rustlang/status/9999?s=20"
        );
    }

    // ── extract_content ───────────────────────────────────────────────────────

    #[test]
    fn extracts_og_description_property_first() {
        let html = r#"<html><head>
            <meta property="og:description" content="Hello tweet!" />
        </head></html>"#;
        assert_eq!(extract_content(html), Some("Hello tweet!".into()));
    }

    #[test]
    fn extracts_og_description_content_first() {
        let html = r#"<html><head>
            <meta content="Content first order" property="og:description" />
        </head></html>"#;
        assert_eq!(extract_content(html), Some("Content first order".into()));
    }

    #[test]
    fn decodes_html_entities_in_content() {
        let html = r#"<meta property="og:description" content="Rock &amp; Roll &#39;til dawn" />"#;
        assert_eq!(
            extract_content(html),
            Some("Rock & Roll 'til dawn".into())
        );
    }

    #[test]
    fn returns_none_when_no_og_description() {
        let html = r#"<html><head><title>No meta here</title></head></html>"#;
        assert_eq!(extract_content(html), None);
    }
}
