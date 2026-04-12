use anyhow::{Result, anyhow};
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use std::sync::LazyLock;

use crate::HttpClient;

/// A single caption line from a YouTube transcript.
#[derive(Debug, PartialEq)]
pub struct TranscriptLine {
    /// Decoded caption text.
    pub text: String,
    /// Start time in seconds.
    pub offset: f64,
    /// Duration in seconds.
    pub duration: f64,
}

/// A full transcript with title and lines.
#[derive(Debug)]
pub struct Transcript {
    pub title: String,
    pub lines: Vec<TranscriptLine>,
}

/// Android client context sent to the Innertube player API.
/// Using the Android client bypasses bot detection that blocks the HTML approach.
const INNERTUBE_CLIENT: &str =
    r#"{"context":{"client":{"clientName":"ANDROID","clientVersion":"20.10.38"}},"videoId":""#;
const INNERTUBE_URL: &str = "https://www.youtube.com/youtubei/v1/player?key=";

static RE_INNERTUBE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""INNERTUBE_API_KEY"\s*:\s*"([A-Za-z0-9_-]+)""#)
        .expect("invalid INNERTUBE_API_KEY regex")
});

static RE_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<title>([^<]*)</title>").expect("invalid title regex"));

/// Extract a YouTube video ID from a URL.
///
/// Returns `None` if the URL does not contain a recognisable video ID.
///
/// # Examples
///
/// ```
/// use agent_fetcher::youtube::extract_video_id;
/// assert_eq!(
///     extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
///     Some("dQw4w9WgXcQ".into())
/// );
/// ```
pub fn extract_video_id(url: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^&?/\s]{11})",
        )
        .expect("invalid YouTube regex")
    });

    RE.captures(url).map(|c| c[1].to_string())
}

/// Fetch the INNERTUBE_API_KEY embedded in the YouTube watch page HTML.
pub fn extract_innertube_key(html: &str) -> Option<String> {
    RE_INNERTUBE_KEY.captures(html).map(|c| c[1].to_string())
}

/// Extract the page title from HTML.
fn extract_title(html: &str) -> Option<String> {
    RE_TITLE.captures(html).map(|c| c[1].trim().to_string())
}

/// Parse the captions track list from an Innertube player API JSON response.
///
/// Returns `(title, caption_url)`. The `&fmt=srv3` suffix is stripped from the
/// caption URL so that YouTube serves plain XML instead of the srv3 format.
pub fn extract_caption_info_from_innertube(
    innertube_json: &serde_json::Value,
    page_title: &str,
    lang: Option<&str>,
) -> Result<(String, String)> {
    let tracks = innertube_json["captions"]["playerCaptionsTracklistRenderer"]["captionTracks"]
        .as_array()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow!("No caption tracks available for this video"))?;

    let track = if let Some(lang_code) = lang {
        tracks
            .iter()
            .find(|t| t["languageCode"].as_str() == Some(lang_code))
            .ok_or_else(|| {
                let available: Vec<&str> = tracks
                    .iter()
                    .filter_map(|t| t["languageCode"].as_str())
                    .collect();
                anyhow!(
                    "Language '{lang_code}' not available. Available: {}",
                    available.join(", ")
                )
            })?
    } else {
        tracks.first().unwrap()
    };

    let raw_url = track["baseUrl"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing baseUrl in caption track"))?;

    // Strip &fmt=srv3 if present; without it YouTube returns plain XML.
    let url = raw_url.replace("&fmt=srv3", "");

    Ok((page_title.to_string(), url))
}

/// Parse YouTube transcript XML into a list of [`TranscriptLine`]s.
///
/// YouTube wraps caption text in `<text start="…" dur="…">…</text>` elements.
/// `quick-xml` automatically decodes XML entities in text content.
pub fn parse_transcript_xml(xml: &str) -> Result<Vec<TranscriptLine>> {
    let mut reader = Reader::from_str(xml);
    let mut lines = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"text" => {
                let mut start = 0.0f64;
                let mut dur = 0.0f64;

                for attr in e.attributes() {
                    let attr = attr?;
                    let value = std::str::from_utf8(attr.value.as_ref())?;
                    match attr.key.as_ref() {
                        b"start" => start = value.parse().unwrap_or(0.0),
                        b"dur" => dur = value.parse().unwrap_or(0.0),
                        _ => {}
                    }
                }

                // The text node immediately follows the opening tag
                if let Event::Text(t) = reader.read_event()? {
                    let text = t.unescape()?.trim().to_string();
                    if !text.is_empty() {
                        lines.push(TranscriptLine {
                            text,
                            offset: start,
                            duration: dur,
                        });
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(lines)
}

/// Fetch the full transcript for a YouTube video via the Innertube player API.
///
/// Flow:
/// 1. GET the watch page to extract `INNERTUBE_API_KEY` and the page title
/// 2. POST to `youtubei/v1/player` with an Android client context
/// 3. Extract the caption track URL from the JSON response
/// 4. GET the transcript XML and parse it
pub async fn fetch_transcript(
    client: &HttpClient,
    video_id: &str,
    lang: Option<&str>,
) -> Result<Transcript> {
    use impit::request::RequestOptions;

    // Step 1: fetch watch page for INNERTUBE_API_KEY + title
    let watch_url = format!("https://www.youtube.com/watch?v={video_id}");
    let page_resp = client
        .get(watch_url, None, None)
        .await
        .map_err(|e| anyhow!("Failed to fetch YouTube page: {e}"))?;
    let page_html = page_resp
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read YouTube page: {e}"))?;

    let api_key = extract_innertube_key(&page_html)
        .ok_or_else(|| anyhow!("INNERTUBE_API_KEY not found in page"))?;
    let title = extract_title(&page_html).unwrap_or_else(|| video_id.to_string());

    // Step 2: POST to Innertube player API with Android client
    let innertube_url = format!("{INNERTUBE_URL}{api_key}");
    let body_json = format!("{INNERTUBE_CLIENT}{video_id}\"}}");
    let body_bytes = body_json.into_bytes();

    let mut headers = vec![("Content-Type".into(), "application/json".into())];
    if let Some(l) = lang {
        headers.push(("Accept-Language".into(), l.into()));
    }
    let options = RequestOptions {
        headers,
        timeout: None,
        http3_prior_knowledge: false,
    };

    let innertube_resp = client
        .post(innertube_url.clone(), Some(body_bytes), Some(options))
        .await
        .map_err(|e| anyhow!("Failed to call Innertube API: {e}"))?;
    let innertube_body = innertube_resp
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read Innertube response: {e}"))?;

    let innertube_json: serde_json::Value = serde_json::from_str(&innertube_body)
        .map_err(|e| anyhow!("Failed to parse Innertube JSON: {e}"))?;

    // Step 3: extract caption URL
    let (title, caption_url) = extract_caption_info_from_innertube(&innertube_json, &title, lang)?;

    // Step 4: fetch and parse the transcript XML
    let xml_resp = client
        .get(caption_url, None, None)
        .await
        .map_err(|e| anyhow!("Failed to fetch transcript XML: {e}"))?;
    let xml = xml_resp
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read transcript XML: {e}"))?;

    let lines = parse_transcript_xml(&xml)?;

    Ok(Transcript { title, lines })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_video_id ─────────────────────────────────────────────────────

    #[test]
    fn video_id_from_watch_url() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn video_id_from_short_url() {
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn video_id_from_embed_url() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn video_id_none_for_homepage() {
        assert_eq!(extract_video_id("https://www.youtube.com/"), None);
    }

    // ── extract_innertube_key ────────────────────────────────────────────────

    #[test]
    fn extracts_innertube_key() {
        let html = r#"var config = {"INNERTUBE_API_KEY": "AIzaSyTest123"};"#;
        assert_eq!(extract_innertube_key(html), Some("AIzaSyTest123".into()));
    }

    // ── extract_caption_info_from_innertube ──────────────────────────────────

    #[test]
    fn extracts_caption_url_from_innertube() {
        let json: serde_json::Value = serde_json::json!({
            "captions": {
                "playerCaptionsTracklistRenderer": {
                    "captionTracks": [
                        {"baseUrl": "https://example.com/timedtext?lang=en", "languageCode": "en"}
                    ]
                }
            }
        });
        let (_, url) = extract_caption_info_from_innertube(&json, "Test Video", None).unwrap();
        assert_eq!(url, "https://example.com/timedtext?lang=en");
    }

    #[test]
    fn strips_fmt_srv3_from_caption_url() {
        let json: serde_json::Value = serde_json::json!({
            "captions": {
                "playerCaptionsTracklistRenderer": {
                    "captionTracks": [
                        {"baseUrl": "https://example.com/timedtext?lang=en&fmt=srv3", "languageCode": "en"}
                    ]
                }
            }
        });
        let (_, url) = extract_caption_info_from_innertube(&json, "Title", None).unwrap();
        assert_eq!(url, "https://example.com/timedtext?lang=en");
    }

    #[test]
    fn selects_language_track() {
        let json: serde_json::Value = serde_json::json!({
            "captions": {
                "playerCaptionsTracklistRenderer": {
                    "captionTracks": [
                        {"baseUrl": "https://example.com/en", "languageCode": "en"},
                        {"baseUrl": "https://example.com/ja", "languageCode": "ja"}
                    ]
                }
            }
        });
        let (_, url) = extract_caption_info_from_innertube(&json, "Title", Some("ja")).unwrap();
        assert_eq!(url, "https://example.com/ja");
    }

    // ── parse_transcript_xml ─────────────────────────────────────────────────

    #[test]
    fn parses_basic_transcript() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
            <transcript>
              <text start="0.0" dur="3.0">Hello world</text>
              <text start="3.0" dur="2.0">How are you?</text>
            </transcript>"#;

        let lines = parse_transcript_xml(xml).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello world");
        assert_eq!(lines[0].offset, 0.0);
        assert_eq!(lines[0].duration, 3.0);
        assert_eq!(lines[1].text, "How are you?");
    }

    #[test]
    fn decodes_xml_entities() {
        let xml = r#"<transcript><text start="0" dur="1">Rock &amp; Roll &#39;til you drop</text></transcript>"#;
        let lines = parse_transcript_xml(xml).unwrap();
        assert_eq!(lines[0].text, "Rock & Roll 'til you drop");
    }

    #[test]
    fn skips_empty_lines() {
        let xml = r#"<transcript>
            <text start="0" dur="1">Hello</text>
            <text start="1" dur="1">   </text>
            <text start="2" dur="1">World</text>
        </transcript>"#;
        let lines = parse_transcript_xml(xml).unwrap();
        assert_eq!(lines.len(), 2);
    }
}
