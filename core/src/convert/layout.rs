//! Where the output goes: one destination per converted file, and the
//! non-audio files that travelled with them.
//!
//! Split out of [`super`] because it answers a different question from the
//! rest of that module. The encoders decide *how* a file is written; this
//! decides *where*, and it is the part users notice when it is wrong — a
//! flattened album is immediately visible, a slightly different LPC order is
//! not.
//!
//! ## Why the base folder is remembered, not computed
//!
//! An earlier version derived one shared root from the file list, as the
//! deepest common ancestor of every source's parent. That is subtly not what
//! the user asked for, and the gap showed up in the most ordinary case there
//! is: drop a band folder holding a single album, and every file shares the
//! parent `Band/Album`, so the "common root" *is* `Band/Album` and stripping
//! it drops both folders — the tracks land flat in the destination, the band
//! and album folders gone. The user dropped a folder; the folder is what they
//! expect to find.
//!
//! So each source carries the base its layout is expressed against, set from
//! what was actually dropped rather than recovered afterwards. Per file, not
//! per batch, which also fixes a second-order oddity: two unrelated folders
//! no longer get forced under whatever distant ancestor they happen to share.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::ConvertFormat;

/// One source file and the folder its layout should be mirrored from.
#[derive(Debug, Clone)]
pub struct ConvertSource {
    /// The audio file to convert.
    pub path: PathBuf,
    /// The folder `path`'s position is expressed relative to — the *parent*
    /// of what was dropped, so a dropped folder is itself recreated at the
    /// destination. For a loose file this is simply its own parent, so it
    /// lands directly in the output folder.
    pub base: PathBuf,
}

/// Work out one destination path per source, reproducing each file's position
/// relative to its own [`ConvertSource::base`] under `output_root` — same
/// folders, same nesting, only the root and the extension change.
///
/// A source that (unexpectedly) doesn't live under its base falls back to
/// just its own file name directly under `output_root`, rather than failing
/// the whole batch over one path oddity: a destination that is merely flatter
/// than intended is a far smaller problem than a batch that refuses to run.
pub fn plan_batch(
    sources: &[ConvertSource],
    output_root: &Path,
    format: ConvertFormat,
) -> Vec<PathBuf> {
    sources
        .iter()
        .map(|s| {
            let rel: &Path = s.path.strip_prefix(&s.base).unwrap_or_else(|_| {
                s.path.file_name().map(Path::new).unwrap_or(s.path.as_path())
            });
            output_root.join(rel).with_extension(format.extension())
        })
        .collect()
}

/// The folders to sweep for non-audio files, paired with the base each one's
/// contents are mirrored against.
///
/// Recovered from the sources rather than passed in: a source's first path
/// component under its base *is* what was dropped. `Band/Album/01.flac`
/// relative to `/music` means `/music/Band` was dropped, so that is the
/// folder to sweep — sweeping `/music` itself would drag in every other band
/// sitting beside it.
fn sweep_roots(sources: &[ConvertSource]) -> HashSet<(PathBuf, PathBuf)> {
    sources
        .iter()
        .filter_map(|s| {
            let rel = s.path.strip_prefix(&s.base).ok()?;
            let mut comps = rel.components();
            let first = comps.next()?;
            // A single component means the file sits directly in `base` — it
            // was dropped as a loose file, so the folder to sweep is `base`.
            let root = if comps.next().is_none() {
                s.base.clone()
            } else {
                s.base.join(first)
            };
            Some((root, s.base.clone()))
        })
        .collect()
}

/// Copy every file that isn't one of the tracks being converted — covers,
/// `.m3u` playlists, generated spectrograms, anything else sharing the
/// folder — to the same relative position under `output_root`.
///
/// "Tout ou rien": there is no per-file choice, by design. The audio files in
/// `sources` are the exclusion list, so a caller cannot get it out of step
/// with what was actually converted.
pub fn passthrough_files(
    sources: &[ConvertSource],
    output_root: &Path,
) -> std::io::Result<Vec<PathBuf>> {
    let audio: HashSet<&Path> = sources.iter().map(|s| s.path.as_path()).collect();
    let mut written = Vec::new();
    // Overlapping drops (a folder *and* something inside it) would otherwise
    // copy the same file twice; the destination is what has to be unique.
    let mut done: HashSet<PathBuf> = HashSet::new();

    for (root, base) in sweep_roots(sources) {
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.into_path();
            if !path.is_file() || audio.contains(path.as_path()) {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&base) else {
                continue;
            };
            let dest = output_root.join(rel);
            if !done.insert(dest.clone()) {
                continue;
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&path, &dest)?;
            written.push(dest);
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(path: &str, base: &str) -> ConvertSource {
        ConvertSource {
            path: PathBuf::from(path),
            base: PathBuf::from(base),
        }
    }

    #[test]
    fn mirrors_the_source_structure_under_the_new_root() {
        let sources = vec![
            src("/music/Album/01 track.mp3", "/music"),
            src("/music/Album/Disc 2/02 track.mp3", "/music"),
        ];
        let got = plan_batch(&sources, Path::new("/export"), ConvertFormat::Flac);
        assert_eq!(
            got,
            vec![
                PathBuf::from("/export/Album/01 track.flac"),
                PathBuf::from("/export/Album/Disc 2/02 track.flac"),
            ]
        );
    }

    /// The regression this whole shape exists for. A band folder holding
    /// exactly one album gives every file the same parent, so a base derived
    /// from the file list would be `/music/Band/Album` and both folders would
    /// vanish — which is precisely what users saw: tracks dumped flat into
    /// the destination.
    #[test]
    fn a_dropped_folder_survives_even_when_it_holds_a_single_album() {
        let sources = vec![
            src("/music/Band/Album/01.flac", "/music"),
            src("/music/Band/Album/02.flac", "/music"),
        ];
        let got = plan_batch(&sources, Path::new("/export"), ConvertFormat::Opus);
        assert_eq!(
            got,
            vec![
                PathBuf::from("/export/Band/Album/01.opus"),
                PathBuf::from("/export/Band/Album/02.opus"),
            ]
        );
    }

    /// Two unrelated folders keep their own names instead of being forced
    /// under whatever distant ancestor they share — with one shared base
    /// these would have come out as `music/A/...` and `elsewhere/B/...`.
    #[test]
    fn unrelated_drops_do_not_grow_a_shared_prefix() {
        let sources = vec![
            src("/music/A/01.flac", "/music"),
            src("/elsewhere/B/01.flac", "/elsewhere"),
        ];
        let got = plan_batch(&sources, Path::new("/export"), ConvertFormat::Mp3);
        assert_eq!(
            got,
            vec![
                PathBuf::from("/export/A/01.mp3"),
                PathBuf::from("/export/B/01.mp3"),
            ]
        );
    }

    /// Loose files mirror nothing: their base is their own parent, so they
    /// land directly in the destination.
    #[test]
    fn dropped_files_land_flat_in_the_destination() {
        let sources = vec![src("/music/Album/01 track.flac", "/music/Album")];
        let got = plan_batch(&sources, Path::new("/export"), ConvertFormat::Wav);
        assert_eq!(got, vec![PathBuf::from("/export/01 track.wav")]);
    }

    #[test]
    fn falls_back_to_the_file_name_for_a_path_outside_its_base() {
        let sources = vec![src("/elsewhere/loose.wav", "/music/Album")];
        let got = plan_batch(&sources, Path::new("/export"), ConvertFormat::Wav);
        assert_eq!(got, vec![PathBuf::from("/export/loose.wav")]);
    }

    #[test]
    fn sweep_root_is_the_dropped_folder_not_its_parent() {
        let sources = vec![src("/music/Band/Album/01.flac", "/music")];
        let roots = sweep_roots(&sources);
        assert_eq!(
            roots.into_iter().collect::<Vec<_>>(),
            vec![(PathBuf::from("/music/Band"), PathBuf::from("/music"))]
        );
    }

    #[test]
    fn sweep_root_of_a_loose_file_is_its_own_folder() {
        let sources = vec![src("/music/Album/01.flac", "/music/Album")];
        let roots = sweep_roots(&sources);
        assert_eq!(
            roots.into_iter().collect::<Vec<_>>(),
            vec![(PathBuf::from("/music/Album"), PathBuf::from("/music/Album"))]
        );
    }

    #[test]
    fn passthrough_copies_everything_except_the_converted_tracks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let music = dir.path().join("music");
        let out = dir.path().join("out");
        std::fs::create_dir_all(music.join("Band/Album/Disc 1")).expect("mkdir");
        for rel in [
            "Band/Album/track.mp3",
            "Band/Album/cover.jpg",
            "Band/Album/Disc 1/playlist.m3u",
        ] {
            std::fs::write(music.join(rel), b"x").expect("write");
        }

        let sources = vec![ConvertSource {
            path: music.join("Band/Album/track.mp3"),
            base: music.clone(),
        }];
        let written = passthrough_files(&sources, &out).expect("copy");

        assert_eq!(written.len(), 2, "{written:?}");
        assert!(out.join("Band/Album/cover.jpg").exists());
        assert!(out.join("Band/Album/Disc 1/playlist.m3u").exists());
        assert!(
            !out.join("Band/Album/track.mp3").exists(),
            "the audio is written by the encoder, not copied"
        );
    }

    /// A folder dropped alongside something inside it must not copy the same
    /// neighbour twice.
    #[test]
    fn overlapping_drops_copy_each_file_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let music = dir.path().join("music");
        let out = dir.path().join("out");
        std::fs::create_dir_all(music.join("Band/Album")).expect("mkdir");
        std::fs::write(music.join("Band/Album/a.flac"), b"x").expect("write");
        std::fs::write(music.join("Band/Album/cover.jpg"), b"x").expect("write");

        // Same file reached through two different drops of the same base.
        let sources = vec![
            ConvertSource {
                path: music.join("Band/Album/a.flac"),
                base: music.clone(),
            },
            ConvertSource {
                path: music.join("Band/Album/a.flac"),
                base: music.join("Band"),
            },
        ];
        let written = passthrough_files(&sources, &out).expect("copy");
        let covers = written
            .iter()
            .filter(|p| p.file_name().is_some_and(|n| n == "cover.jpg"))
            .count();
        assert_eq!(covers, 1, "{written:?}");
    }
}
