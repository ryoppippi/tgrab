use anyhow::{anyhow, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

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

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

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
    use regex::Regex;
    use std::sync::LazyLock;

    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^&?/\s]{11})",
        )
        .expect("invalid YouTube regex")
    });

    RE.captures(url).map(|c| c[1].to_string())
}

/// Extract the page title and the best caption track URL from raw YouTube page HTML.
///
/// Returns `Err` when transcripts are disabled or unavailable.
pub fn extract_caption_info(html: &str, lang: Option<&str>) -> Result<(String, String)> {
    // Title
    let title = {
        use regex::Regex;
        use std::sync::LazyLock;
        static RE_TITLE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<title>([^<]*)</title>").expect("invalid title regex"));
        RE_TITLE
            .captures(html)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_else(|| "Unknown".into())
    };

    if html.contains("class=\"g-recaptcha\"") {
        anyhow::bail!("YouTube rate-limit: CAPTCHA required");
    }

    // Split on the captions JSON key
    let parts: Vec<&str> = html.splitn(2, "\"captions\":").collect();
    if parts.len() < 2 {
        if !html.contains("\"playabilityStatus\":") {
            anyhow::bail!("Video unavailable");
        }
        anyhow::bail!("Transcript is disabled for this video");
    }

    // Normalise newlines to spaces so the regex can match across line boundaries.
    let rest = parts[1].replace('\n', " ");

    // Use a regex to tolerate optional whitespace between the comma and the key.
    use regex::Regex;
    use std::sync::LazyLock;
    static RE_VD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#",\s*"videoDetails""#).expect("invalid regex"));

    let captions_raw = RE_VD
        .find(&rest)
        .map(|m| rest[..m.start()].trim().to_string())
        .ok_or_else(|| anyhow!("Malformed captions JSON: videoDetails separator not found"))?;

    let captions: serde_json::Value = serde_json::from_str(&captions_raw)?;
    let renderer = &captions["playerCaptionsTracklistRenderer"];

    if renderer.is_null() {
        anyhow::bail!("Transcript is disabled for this video");
    }

    let tracks = renderer["captionTracks"]
        .as_array()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow!("No transcripts available for this video"))?;

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

    let url = track["baseUrl"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing baseUrl in caption track"))?
        .to_string();

    Ok((title, url))
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

/// Fetch the full transcript for a YouTube video.
///
/// # Arguments
///
/// * `client` – shared impit HTTP client
/// * `video_id` – 11-character YouTube video ID
/// * `lang` – optional BCP-47 language code (e.g. `"en"`, `"ja"`)
pub async fn fetch_transcript(
    client: &HttpClient,
    video_id: &str,
    lang: Option<&str>,
) -> Result<Transcript> {
    use impit::request::RequestOptions;

    let url = format!("https://www.youtube.com/watch?v={video_id}");

    let options = RequestOptions {
        headers: {
            let mut h = vec![("User-Agent".into(), USER_AGENT.into())];
            if let Some(l) = lang {
                h.push(("Accept-Language".into(), l.into()));
            }
            h
        },
        timeout: None,
        http3_prior_knowledge: false,
    };

    let response = client
        .get(url, None, Some(options))
        .await
        .map_err(|e| anyhow!("HTTP error fetching YouTube page: {e}"))?;

    let html = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read YouTube page body: {e}"))?;

    let (title, transcript_url) = extract_caption_info(&html, lang)?;

    let xml_response = client
        .get(transcript_url, None, None)
        .await
        .map_err(|e| anyhow!("HTTP error fetching transcript XML: {e}"))?;

    let xml = xml_response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read transcript XML body: {e}"))?;

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

    // ── extract_caption_info ─────────────────────────────────────────────────

    fn mock_youtube_html(base_url: &str, lang: &str) -> String {
        format!(
            r#"<html><head><title>Test Video - YouTube</title></head><body>
            <script>
            var ytInitialPlayerResponse = {{
                "captions":{{"playerCaptionsTracklistRenderer":{{"captionTracks":[
                    {{"baseUrl":"{base_url}","languageCode":"{lang}"}}
                ]}}}},
                "videoDetails":{{"videoId":"abc"}}
            }};
            </script></body></html>"#
        )
    }

    #[test]
    fn extracts_title_and_caption_url() {
        let html = mock_youtube_html("https://example.com/transcript", "en");
        let (title, url) = extract_caption_info(&html, None).unwrap();
        assert_eq!(title, "Test Video - YouTube");
        assert_eq!(url, "https://example.com/transcript");
    }

    #[test]
    fn selects_language_track() {
        let html = format!(
            r#"<title>V</title>
            "captions":{{"playerCaptionsTracklistRenderer":{{"captionTracks":[
                {{"baseUrl":"https://example.com/en","languageCode":"en"}},
                {{"baseUrl":"https://example.com/ja","languageCode":"ja"}}
            ]}}}},
            "videoDetails":{{}}"#
        );
        let (_, url) = extract_caption_info(&html, Some("ja")).unwrap();
        assert_eq!(url, "https://example.com/ja");
    }

    #[test]
    fn err_when_no_captions_key() {
        let html = r#"<html><body>"playabilityStatus":{}</body></html>"#;
        assert!(extract_caption_info(html, None).is_err());
    }

    #[test]
    fn err_when_video_unavailable() {
        let html = r#"<html><body>no captions here</body></html>"#;
        let err = extract_caption_info(html, None).unwrap_err();
        assert!(err.to_string().contains("unavailable"));
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
