//! HTTP plumbing shared by both lookup providers: the client (and its
//! timeout), the User-Agent both APIs' policies ask for, and the cover-image
//! download with its size cap.

use flaccompagnon_services::tags::CoverArt;

/// Every request gets a timeout: without one, an unresponsive server leaves
/// the search pop-in spinning forever with no way out but restarting the
/// app. 20s is generous for these APIs while still bounded.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Cap on a downloaded cover image. Cover Art Archive's `front-500` is a few
/// hundred KB; Discogs' originals can be much larger. Whatever comes back is
/// base64'd and handed to the webview, so an unbounded download would be a
/// memory blowup driven by a third-party response.
const MAX_COVER_BYTES: usize = flaccompagnon_services::tags::cover::MAX_COVER_BYTES;
/// A detailed release is much smaller than this even with a large track list;
/// malformed API responses must not allocate without a bound either.
const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;

/// MusicBrainz's usage policy requires a descriptive User-Agent with a way to
/// reach the developer; Discogs asks for the same courtesy.
pub(super) fn user_agent() -> String {
    format!(
        "FlacCompagnon/{} (+https://github.com/craft-and-code/FlacCompagnon)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Shared client builder — one place for the timeout so no call site can
/// forget it.
pub(super) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // Applied to redirects as well as the original URL, so artwork hosted
        // elsewhere cannot downgrade a request to cleartext HTTP.
        .https_only(true)
        .build()
        .map_err(|e| format!("Could not create the HTTP client: {e}"))
}

/// Turn a provider's response into parsed JSON, or an error message naming
/// the provider — the "make sure it actually succeeded, then parse it" step
/// every search/detail call repeats, whichever provider it talks to.
pub(super) async fn parse_json_response(
    resp: reqwest::Response,
    provider: &str,
) -> Result<serde_json::Value, String> {
    if !resp.status().is_success() {
        return Err(format!("{provider} returned {}", resp.status()));
    }
    let bytes = read_bounded_response(resp, MAX_JSON_BYTES)
        .await
        .map_err(|e| format!("{provider} response could not be read: {e}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("{provider} response was not valid JSON: {e}"))
}

async fn read_bounded_response(
    mut resp: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    if resp.content_length().is_some_and(|len| len > limit as u64) {
        return Err("response exceeds the size limit".to_string());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        append_bounded(&mut bytes, &chunk, limit)?;
    }
    Ok(bytes)
}

fn append_bounded(bytes: &mut Vec<u8>, chunk: &[u8], limit: usize) -> Result<(), String> {
    if chunk.len() > limit.saturating_sub(bytes.len()) {
        return Err("response exceeds the size limit".to_string());
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

/// Best-effort image download — used for both providers' cover art. `None`
/// on any failure (missing image, network error, unrecognized format): the
/// rest of the lookup result is still useful without a cover.
pub(super) async fn fetch_cover(client: &reqwest::Client, url: &str) -> Option<CoverArt> {
    // The Discogs branch takes this URL straight from an API response, so
    // it isn't inherently trusted: anything but plain HTTPS is refused
    // rather than followed.
    if !url.starts_with("https://") {
        return None;
    }
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent())
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = read_bounded_response(resp, MAX_COVER_BYTES).await.ok()?;
    flaccompagnon_services::tags::cover_from_bytes(bytes, url).ok()
}

#[cfg(test)]
#[path = "../../tests/unit/lookup/http.rs"]
mod tests;
