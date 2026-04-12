use anyhow::{Result, anyhow};
use regex::Regex;
use std::sync::LazyLock;

/// Which service a URL belongs to.
#[derive(Debug, PartialEq)]
pub enum Service {
    /// YouTube video; contains the 11-character video ID.
    YouTube(String),
    /// Twitter / X post; original URL preserved for rewriting later.
    Twitter(String),
    /// Bluesky post; original URL preserved for rewriting later.
    Bluesky(String),
}

static RE_YOUTUBE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^&?/\s]{11})",
    )
    .expect("invalid YouTube regex")
});

static RE_TWITTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:www\.)?(?:twitter\.com|x\.com)/\w+/status/\d+")
        .expect("invalid Twitter regex")
});

static RE_BLUESKY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?bsky\.app/profile/[^/]+/post/[A-Za-z0-9]+")
        .expect("invalid Bluesky regex")
});

/// Inspect `url` and return which [`Service`] should handle it.
///
/// # Errors
///
/// Returns an error if the URL does not match any supported service.
///
/// # Examples
///
/// ```
/// use tgrab::router::{route, Service};
/// assert_eq!(
///     route("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
///     Service::YouTube("dQw4w9WgXcQ".into())
/// );
/// ```
pub fn route(url: &str) -> Result<Service> {
    if let Some(caps) = RE_YOUTUBE.captures(url) {
        return Ok(Service::YouTube(caps[1].to_string()));
    }
    if RE_TWITTER.is_match(url) {
        return Ok(Service::Twitter(url.to_string()));
    }
    if RE_BLUESKY.is_match(url) {
        return Ok(Service::Bluesky(url.to_string()));
    }
    Err(anyhow!("Unsupported URL: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── YouTube ──────────────────────────────────────────────────────────────

    #[test]
    fn youtube_standard_watch() {
        assert_eq!(
            route("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_no_www() {
        assert_eq!(
            route("https://youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_short_url() {
        assert_eq!(
            route("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_embed() {
        assert_eq!(
            route("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_v_path() {
        assert_eq!(
            route("https://www.youtube.com/v/dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_mobile() {
        assert_eq!(
            route("https://m.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn youtube_with_timestamp() {
        assert_eq!(
            route("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s").unwrap(),
            Service::YouTube("dQw4w9WgXcQ".into())
        );
    }

    // ── Twitter / X ──────────────────────────────────────────────────────────

    #[test]
    fn twitter_x_com() {
        assert_eq!(
            route("https://x.com/user/status/1234567890").unwrap(),
            Service::Twitter("https://x.com/user/status/1234567890".into())
        );
    }

    #[test]
    fn twitter_twitter_com() {
        assert_eq!(
            route("https://twitter.com/user/status/1234567890").unwrap(),
            Service::Twitter("https://twitter.com/user/status/1234567890".into())
        );
    }

    #[test]
    fn twitter_www_prefix() {
        assert_eq!(
            route("https://www.twitter.com/user/status/1234567890").unwrap(),
            Service::Twitter("https://www.twitter.com/user/status/1234567890".into())
        );
    }

    // ── Bluesky ──────────────────────────────────────────────────────────────

    #[test]
    fn bluesky_post() {
        assert_eq!(
            route("https://bsky.app/profile/user.bsky.social/post/abc123").unwrap(),
            Service::Bluesky("https://bsky.app/profile/user.bsky.social/post/abc123".into())
        );
    }

    #[test]
    fn bluesky_did_profile() {
        // DIDs contain colons which are valid in URL path segments
        assert_eq!(
            route("https://bsky.app/profile/did:plc:abc123/post/xyz789").unwrap(),
            Service::Bluesky("https://bsky.app/profile/did:plc:abc123/post/xyz789".into())
        );
    }

    // ── Unsupported ───────────────────────────────────────────────────────────

    #[test]
    fn unsupported_url_returns_err() {
        assert!(route("https://example.com/page").is_err());
    }

    #[test]
    fn bare_youtube_homepage_returns_err() {
        assert!(route("https://www.youtube.com/").is_err());
    }
}
