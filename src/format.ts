// Pure, DOM-free helpers for formatting values.
//
// This file used to also build table-cell HTML as strings, which meant every
// value that reached the page had to be run through an `escapeHtml` by hand.
// Components render that markup now, and JSX escapes interpolated values
// itself, so that whole class of mistake is gone along with the helper.

import type { CoverArt, FileAnalysis } from "./types";

/// Audio extensions, stripped when suggesting a file name from a dropped file.
const AUDIO_EXTS = [
  "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf",
  "ogg", "oga", "mp3", "aac",
];

/// Image MIME types a cover may legitimately declare. Anything else is treated
/// as "no usable image" rather than passed through.
const SAFE_IMAGE_MIMES = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/bmp",
  "image/webp",
  "image/tiff",
];

/// A `data:` URL for a cover.
///
/// The MIME string comes from the audio file's own tag — i.e. from a file the
/// user merely opened — so it is not trusted: only a known image type is
/// accepted, and `null` means "don't render an image". (`data_base64` needs no
/// such care: the backend base64-encodes it, so it can only ever contain
/// `A-Za-z0-9+/=`.)
export function coverDataUrl(cover: CoverArt): string | null {
  const mime = cover.mime.trim().toLowerCase();
  if (!SAFE_IMAGE_MIMES.includes(mime)) return null;
  return `data:${mime};base64,${cover.data_base64}`;
}

export function fmtDuration(secs: number): string {
  const t = Math.round(secs);
  const m = Math.floor(t / 60);
  const s = t % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

/// File size from the exact byte count the Rust core read off the filesystem.
///
/// Uses **decimal** units (1 kB = 1000 bytes), which is what macOS Finder and
/// most Linux file managers display — so the number matches what the OS shows
/// for the same file. Windows Explorer instead labels *binary* units "KB"/"MB",
/// so it reads slightly smaller there; switching `STEP` to 1024 and the labels
/// to KiB/MiB is the one-line change if that convention is ever preferred.
export function fmtSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const STEP = 1000;
  if (bytes < STEP) return `${bytes} B`;
  const units = ["kB", "MB", "GB", "TB"];
  let value = bytes / STEP;
  let unit = 0;
  while (value >= STEP && unit < units.length - 1) {
    value /= STEP;
    unit++;
  }
  // Sub-MB sizes are shown whole (Finder-style "573 kB"); larger ones keep a
  // decimal so a 32.3 MB track doesn't collapse to a bare "32 MB".
  const digits = unit === 0 ? 0 : value < 100 ? 1 : 0;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export function fmtCutoff(f: FileAnalysis): string {
  if (f.cutoff_hz == null || f.cutoff_ratio == null) return "—";
  return `${(f.cutoff_hz / 1000).toFixed(1)} kHz (${Math.round(f.cutoff_ratio * 100)}%)`;
}

/// `CoverArt.picture_type` is Rust's `Debug` output for lofty's `PictureType`
/// enum (e.g. `"CoverFront"`). These keys are exactly the strings
/// `core::tags::parse_picture_type` understands on the Rust side; anything
/// else falls back to "Other" there too.
export const PICTURE_TYPE_LABELS: Record<string, string> = {
  CoverFront: "Front cover",
  CoverBack: "Back cover",
  Icon: "Icon",
  OtherIcon: "Other icon",
  Leaflet: "Leaflet page",
  Media: "Media (label/disc)",
  LeadArtist: "Lead artist",
  Artist: "Artist",
  Conductor: "Conductor",
  Band: "Band/orchestra",
  Composer: "Composer",
  Lyricist: "Lyricist/writer",
  RecordingLocation: "Recording location",
  DuringRecording: "During recording",
  DuringPerformance: "During performance",
  ScreenCapture: "Movie/video screen capture",
  BrightFish: "Bright colored fish",
  Illustration: "Illustration",
  BandLogo: "Band/artist logo",
  PublisherLogo: "Publisher/studio logo",
  Other: "Other",
};

export function pictureTypeLabel(raw: string): string {
  return PICTURE_TYPE_LABELS[raw] ?? raw;
}

/// The deepest folder containing every one of `paths`. With a single folder of
/// files this is that folder; across several folders it is their common
/// ancestor. Recomputed as files are added.
export function commonDir(paths: string[]): string {
  if (paths.length === 0) return "";
  const sep = paths[0].includes("\\") ? "\\" : "/";
  const dirs = paths.map((p) => p.split(/[\\/]/).slice(0, -1)); // drop the filename
  let common = dirs[0];
  for (const d of dirs.slice(1)) {
    let i = 0;
    while (i < common.length && i < d.length && common[i] === d[i]) i++;
    common = common.slice(0, i);
  }
  return common.join(sep) || sep;
}

function nameFrom(path: string, fallback: string): string {
  const segments = path.split(/[\\/]/).filter(Boolean);
  let name = segments.length ? segments[segments.length - 1] : fallback;
  const m = name.match(/\.([A-Za-z0-9]+)$/);
  if (m && AUDIO_EXTS.includes(m[1].toLowerCase())) {
    name = name.slice(0, -(m[1].length + 1));
  }
  return name || fallback;
}

/// Suggested report file name from a single dropped path (folder or file name,
/// with an audio extension stripped).
///
/// `ext` must match the format actually being written. The save dialog appends
/// its own extension when the name it's given doesn't already carry the right
/// one, so offering `Album.csv` while saving JSON produced `Album.csv.json`.
export function reportNameFrom(
  path: string,
  ext: "csv" | "json" = "csv",
  fallback = "FlacCompagnon",
): string {
  return `${nameFrom(path, fallback)}.${ext}`;
}

/// Same idea for a playlist. Extended M3U conventionally uses `.m3u8` (UTF-8),
/// Simple sticks to the plain `.m3u`.
export function playlistNameFrom(
  path: string,
  ext: "m3u8" | "m3u" = "m3u8",
  fallback = "Playlist",
): string {
  return `${nameFrom(path, fallback)}.${ext}`;
}
