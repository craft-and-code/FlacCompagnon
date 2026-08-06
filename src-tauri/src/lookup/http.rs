//! HTTP plumbing shared by both lookup providers: the client (and its
//! timeout), the User-Agent both APIs' policies ask for, and the cover-image
//! download with its size cap.

use flaccompagnon_core::tags::CoverArt;

/// Every request gets a timeout: without one, an unresponsive server leaves
/// the search pop-in spinning forever with no way out but restarting the
/// app. 20s is generous for these APIs while still bounded.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Cap on a downloaded cover image. Cover Art Archive's `front-500` is a few
/// hundred KB; Discogs' originals can be much larger. Whatever comes back is
/// base64'd and handed to the webview, so an unbounded download would be a
/// memory blowup driven by a third-party response.
const MAX_COVER_BYTES: usize = 12 * 1024 * 1024;

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
        .build()
        .map_err(|e| format!("Could not create the HTTP client: {e}"))
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
    // Reject an oversized image up front when the server declares its size,
    // so the common case never starts the download at all.
    if let Some(len) = resp.content_length() {
        if len > MAX_COVER_BYTES as u64 {
            return None;
        }
    }
    let bytes = resp.bytes().await.ok()?;
    // A server can lie about (or omit) Content-Length, so the real size is
    // checked again once the body is in hand.
    if bytes.len() > MAX_COVER_BYTES {
        return None;
    }
    flaccompagnon_core::tags::cover_from_bytes(bytes.to_vec(), url).ok()
}
