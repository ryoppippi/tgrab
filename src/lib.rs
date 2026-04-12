pub mod bluesky;
pub mod router;
pub mod twitter;
pub mod youtube;

use impit::{cookie::Jar, fingerprint::database as fingerprints, impit::Impit};

/// Shared HTTP client type: impit with browser fingerprinting and cookie support.
pub type HttpClient = Impit<Jar>;

/// Create an [`HttpClient`] with Chrome 131 browser fingerprinting.
///
/// Chrome 131 is used to match the User-Agent header sent in YouTube requests.
/// An explicit 30-second timeout prevents the client from hanging indefinitely.
pub fn create_client() -> anyhow::Result<HttpClient> {
    Impit::<Jar>::builder()
        .with_fingerprint(fingerprints::chrome_131::fingerprint())
        .with_default_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {e}"))
}
