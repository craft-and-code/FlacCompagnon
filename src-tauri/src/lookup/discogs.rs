//! Discogs provider. Requires a personal access token (Settings → Developers
//! on discogs.com) — the frontend keeps it in `localStorage` and passes it in
//! on every call; nothing here stores it.

use super::http::{fetch_cover, http_client, user_agent};
use super::{LookupCandidate, LookupRelease, LookupTrack};

/// Results per search page — matches MusicBrainz's limit so the pop-in reads
/// the same whichever provider answered.
const SEARCH_PER_PAGE: &str = "15";

/// Discogs release ids are plain integers. Same reasoning as MusicBrainz's
/// `is_valid_mbid`: the id lands in a URL path, so its shape is checked
/// rather than trusted.
fn is_valid_release_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

/// Search Discogs releases matching free-text `query`.
pub async fn discogs_search(query: String, token: String) -> Result<Vec<LookupCandidate>, String> {
    let client = http_client()?;
    let resp = client
        .get("https://api.discogs.com/database/search")
        .header(reqwest::header::USER_AGENT, user_agent())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Discogs token={token}"),
        )
        .query(&[
            ("q", query.as_str()),
            ("type", "release"),
            ("per_page", SEARCH_PER_PAGE),
        ])
        .send()
        .await
        .map_err(|e| format!("Discogs request failed: {e}"))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Discogs rejected the API token — check it in the search panel.".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("Discogs returned {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Discogs response was not valid JSON: {e}"))?;

    let mut out = Vec::new();
    for r in body
        .get("results")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let Some(id) = r.get("id").and_then(|v| v.as_u64()) else {
            continue;
        };
        // Discogs search results combine "Artist - Title" into one string
        // rather than separate fields; shown as-is by the frontend rather
        // than guessing where to split it (artist names can contain " - ").
        let title = r
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        // Discogs is inconsistent here: search results carry `year` as a
        // string, the release endpoint as a number. Accept either rather than
        // silently dropping the year for half the responses.
        let year = r
            .get("year")
            .and_then(|v| {
                v.as_str()
                    .map(str::to_string)
                    .or_else(|| v.as_u64().map(|y| y.to_string()))
            })
            .filter(|s| !s.is_empty() && s != "0");
        out.push(LookupCandidate {
            source: "Discogs".to_string(),
            id: id.to_string(),
            title,
            artist: String::new(),
            year,
            track_count: None,
        });
    }
    Ok(out)
}

/// Full track list for a Discogs release, plus its primary image if any.
pub async fn discogs_detail(id: String, token: String) -> Result<LookupRelease, String> {
    if !is_valid_release_id(&id) {
        return Err("Not a valid Discogs release ID.".to_string());
    }
    let client = http_client()?;
    let resp = client
        .get(format!("https://api.discogs.com/releases/{id}"))
        .header(reqwest::header::USER_AGENT, user_agent())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Discogs token={token}"),
        )
        .send()
        .await
        .map_err(|e| format!("Discogs request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Discogs returned {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Discogs response was not valid JSON: {e}"))?;

    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown title")
        .to_string();
    let artist = body
        .get("artists")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown artist")
        .to_string();
    let year = body
        .get("year")
        .and_then(|v| v.as_u64())
        .filter(|&y| y > 0)
        .map(|y| y.to_string());

    let mut tracks = Vec::new();
    for t in body
        .get("tracklist")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        // Discogs' tracklist also carries non-track entries (headings,
        // index tracks) tagged via `type_`; only actual tracks are kept.
        let kind = t.get("type_").and_then(|v| v.as_str()).unwrap_or("track");
        if kind != "track" {
            continue;
        }
        let position = t
            .get("position")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let ttitle = t
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !ttitle.is_empty() {
            tracks.push(LookupTrack {
                position,
                title: ttitle,
            });
        }
    }

    let cover_url = body
        .get("images")
        .and_then(|v| v.as_array())
        .and_then(|imgs| {
            imgs.iter()
                .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("primary"))
                .or_else(|| imgs.first())
        })
        .and_then(|i| i.get("resource_url").or_else(|| i.get("uri")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let cover = match cover_url {
        Some(url) => fetch_cover(&client, &url).await,
        None => None,
    };

    Ok(LookupRelease {
        title,
        artist,
        year,
        tracks,
        cover,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_numeric_release_id() {
        assert!(is_valid_release_id("249504"));
        assert!(is_valid_release_id("1"));
    }

    /// The id is interpolated into a URL path, so anything that could steer
    /// the request elsewhere has to be refused before it reaches the network.
    #[test]
    fn rejects_ids_that_could_steer_the_request_elsewhere() {
        assert!(!is_valid_release_id(""));
        assert!(!is_valid_release_id("../../etc/passwd"));
        assert!(!is_valid_release_id("249504/../x"));
        assert!(!is_valid_release_id("249504?token=x"));
        assert!(!is_valid_release_id("249504#f"));
        assert!(!is_valid_release_id("249 504"));
        assert!(!is_valid_release_id("-1"));
    }
}
