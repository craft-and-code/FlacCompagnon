//! MusicBrainz provider. No API key needed; MusicBrainz asks unauthenticated
//! clients to stay around 1 request/second, which a single "Search" click
//! naturally respects. Cover art comes from the Cover Art Archive, keyed by
//! the same release id.

use super::http::{fetch_cover, http_client, parse_json_response, user_agent};
use super::{year_prefix, LookupCandidate, LookupRelease, LookupTrack};

/// How many search results to ask for. Enough to find the right release
/// without turning the pop-in into a list nobody reads.
const SEARCH_LIMIT: &str = "15";

/// The first artist credit's display name — MusicBrainz's `artist-credit`
/// is an array (a release can have several credited artists joined by
/// `joinphrase`); only the first is used, which is enough to tell releases
/// apart and matches what gets written to the `artist` tag.
fn first_artist_credit(v: &serde_json::Value) -> Option<&str> {
    v.get("artist-credit")?
        .as_array()?
        .first()?
        .get("name")?
        .as_str()
}

/// True for a canonical MusicBrainz MBID (a lowercase-or-uppercase UUID).
///
/// Both call sites interpolate the id straight into a URL path, and the id
/// can come from a *file's own tags* (the "already tagged by Picard"
/// shortcut) — i.e. from an untrusted file the user merely opened. Without
/// this check a crafted tag value like `../../something` or one containing a
/// `?`/`#` could steer the request to a different endpoint. Validating the
/// shape here keeps that impossible rather than relying on URL escaping.
fn is_valid_mbid(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, &b)| match i {
        8 | 13 | 18 | 23 => b == b'-',
        _ => b.is_ascii_hexdigit(),
    })
}

/// Search MusicBrainz releases matching free-text `query` (e.g. "radiohead
/// ok computer").
pub async fn musicbrainz_search(query: String) -> Result<Vec<LookupCandidate>, String> {
    let client = http_client()?;
    let resp = client
        .get("https://musicbrainz.org/ws/2/release/")
        .header(reqwest::header::USER_AGENT, user_agent())
        .query(&[
            ("query", query.as_str()),
            ("fmt", "json"),
            ("limit", SEARCH_LIMIT),
        ])
        .send()
        .await
        .map_err(|e| format!("MusicBrainz request failed: {e}"))?;
    let body = parse_json_response(resp, "MusicBrainz").await?;

    let mut out = Vec::new();
    for r in body
        .get("releases")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let Some(id) = r.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let title = r
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown title")
            .to_string();
        let artist = first_artist_credit(r)
            .unwrap_or("Unknown artist")
            .to_string();
        let year = r.get("date").and_then(|v| v.as_str()).and_then(year_prefix);
        let track_count = r.get("media").and_then(|v| v.as_array()).map(|media| {
            media
                .iter()
                .filter_map(|m| m.get("track-count").and_then(|v| v.as_u64()))
                .sum::<u64>() as u32
        });
        out.push(LookupCandidate {
            source: "MusicBrainz".to_string(),
            id: id.to_string(),
            title,
            artist,
            year,
            track_count: track_count.filter(|&n| n > 0),
        });
    }
    Ok(out)
}

/// Full track list for a MusicBrainz release, plus its Cover Art Archive
/// front cover if one exists.
pub async fn musicbrainz_detail(mbid: String) -> Result<LookupRelease, String> {
    if !is_valid_mbid(&mbid) {
        return Err("Not a valid MusicBrainz release ID.".to_string());
    }
    let client = http_client()?;
    // `inc` is built into the URL rather than passed through `.query()`:
    // its sub-values are separated by a literal `+`, which the query
    // serializer would percent-encode to `%2B` — MusicBrainz splits on the
    // raw `+` and rejects the request otherwise.
    let resp = client
        .get(format!(
            "https://musicbrainz.org/ws/2/release/{mbid}?inc=recordings+artist-credits&fmt=json"
        ))
        .header(reqwest::header::USER_AGENT, user_agent())
        .send()
        .await
        .map_err(|e| format!("MusicBrainz request failed: {e}"))?;
    let body = parse_json_response(resp, "MusicBrainz").await?;

    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown title")
        .to_string();
    let artist = first_artist_credit(&body)
        .unwrap_or("Unknown artist")
        .to_string();
    let year = body
        .get("date")
        .and_then(|v| v.as_str())
        .and_then(year_prefix);

    let mut tracks = Vec::new();
    for medium in body
        .get("media")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        for t in medium
            .get("tracks")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            let position = t
                .get("number")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let ttitle = t
                .get("title")
                .or_else(|| t.get("recording").and_then(|r| r.get("title")))
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
    }

    // Safe to interpolate: `mbid` was shape-checked above.
    let cover = fetch_cover(
        &client,
        &format!("https://coverartarchive.org/release/{mbid}/front-500"),
    )
    .await;

    Ok(LookupRelease {
        title,
        artist,
        year,
        tracks,
        cover,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/lookup/musicbrainz.rs"]
mod tests;
