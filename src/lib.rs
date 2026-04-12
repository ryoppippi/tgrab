pub mod bluesky;
pub mod router;
pub mod twitter;
pub mod youtube;

use impit::{fingerprint::database as fingerprints, impit::Impit};

/// Shared HTTP client type: impit with Firefox fingerprinting and cookie support.
pub type HttpClient = Impit<reqwest::cookie::Jar>;

/// Create an [`HttpClient`] with Firefox 144 browser fingerprinting.
pub fn create_client() -> anyhow::Result<HttpClient> {
    Impit::<reqwest::cookie::Jar>::builder()
        .with_fingerprint(fingerprints::firefox_144::fingerprint())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {e}"))
}
