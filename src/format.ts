// Pure, DOM-free helpers for formatting values.
//
// This file used to also build table-cell HTML as strings, which meant every
// value that reached the page had to be run through an `escapeHtml` by hand.
// Components render that markup now, and JSX escapes interpolated values
// itself, so that whole class of mistake is gone along with the helper.

import type { CoverArt, Detections, FileAnalysis, FlacMd5Status } from "./types";

/// Audio extensions, stripped when suggesting a file name from a dropped
/// file and used by [`isAudioPath`]. Mirrors `SUPPORTED_EXTENSIONS` in
/// `core/src/scan.rs` — the backend decides what it can actually decode, this
/// copy only exists so a drop can be refused without a round trip.
const AUDIO_EXTS = [
  "flac", "wav", "wave", "aif", "aiff", "aifc", "alac", "m4a", "mp4", "caf",
  "ogg", "oga", "opus", "mp3", "aac", "dsf", "dff",
];

/// Image extensions a dropped cover may have — see [`isImagePath`].
const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// The lowercase extension of `path`, or `null` when it has none — which for
/// a dropped path most often means it's a folder.
function extOf(path: string): string | null {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : null;
}

/// Whether a dropped path could hold audio: a known audio extension, or no
/// extension at all (a folder, which the backend will walk). Deliberately
/// permissive — this only exists to catch the obvious mistake (dropping an
/// image on an audio target); deciding what's really decodable is the
/// backend's job, from the file's magic bytes rather than its name.
export function isAudioPath(path: string): boolean {
  const ext = extOf(path);
  return ext == null || AUDIO_EXTS.includes(ext);
}

/// Whether a dropped path looks like an image, for the cover drop target.
export function isImagePath(path: string): boolean {
  const ext = extOf(path);
  return ext != null && IMAGE_EXTS.includes(ext);
}

/// The image extensions [`isImagePath`] accepts, for error messages.
export const IMAGE_EXT_LIST = IMAGE_EXTS.join(", ");

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

/// Same idea as `fmtDuration`, but for a summed total (the footer's "N files,
/// H:MM:SS, X GB" stats) rather than a single track — hours are broken out
/// once the total reaches 60 minutes, since a whole library reading
/// "743:12" is far harder to place than "12:23:12".
export function fmtDurationLong(secs: number): string {
  const t = Math.round(secs);
  const h = Math.floor(t / 3600);
  const m = Math.floor((t % 3600) / 60);
  const s = t % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
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

export function fmtBitrate(kbps: number | null): string {
  if (kbps == null) return "—";
  return `${kbps} kbps`;
}

/// Locale-formatted date from a Unix-seconds timestamp (`FileAnalysis.
/// modified_unix`) — no time-of-day component, since the column is about
/// "which file changed last", not a precise moment.
export function fmtModified(unixSecs: number | null): string {
  if (unixSecs == null) return "—";
  return new Date(unixSecs * 1000).toLocaleDateString();
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

/// A path's final component (file or folder name), the same split
/// `useNativeDrop` already did inline for a dropped cover image's extension —
/// centralized here now that the conversion panel's imported-items list needs
/// the same thing for a whole path, not just its extension.
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/// Splits a file name into its editable stem and its extension (dot
/// included, e.g. ".flac"), for the results table's inline rename field —
/// only the stem is editable there, the extension shows next to it as plain
/// text. Mirrors `Path::extension()`'s rule on the Rust side (rename.rs)
/// exactly, so what's pre-filled here is exactly what a resubmit without
/// changes round-trips back to: a name that's only a leading dot with no
/// other dot (".hidden") has no extension, anything else splits on the last
/// dot. This is purely a display convenience — the actual extension the file
/// ends up with after a rename is decided server-side from the file's real
/// current path, never from anything this function returns.
export function splitStem(fileName: string): { stem: string; ext: string } {
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0 || dot === fileName.length - 1) return { stem: fileName, ext: "" };
  return { stem: fileName.slice(0, dot), ext: fileName.slice(dot) };
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

// --- Search filter (TopBar) --------------------------------------------------

function detectionSearchWords(d: Detections): string {
  const tags: string[] = [];
  if (d.upscaling) tags.push("upscaled");
  if (d.upsampling) tags.push("upsampled");
  if (d.transcoding === "detected") tags.push("transcoded");
  else if (d.transcoding === "suspected") tags.push("transcoded suspected");
  if (tags.length === 0) tags.push("clean");
  return tags.join(" ");
}

function md5SearchWords(m: FlacMd5Status | null): string {
  if (!m) return "";
  switch (m.state) {
    case "Match":
      return "md5 ok match";
    case "Mismatch":
      return "md5 mismatch";
    case "NoSignature":
      return "";
    case "Present":
      return "md5 present";
    case "Error":
      return `md5 error ${m.detail}`;
  }
}

/// Flattens everything a results-table row actually shows — file name,
/// format, bit depth, sample rate, detections, clipping, MD5 status, and so
/// on — into one lowercase string for the search filter (TopBar) to match
/// against, so "24-bit" or "transcoded" finds files without needing the
/// filename itself to mention them. Mirrors ResultRow/ResultCells' own
/// formatting rather than sharing code with them (those compute a colour and
/// a tooltip alongside the text, which the filter has no use for) — a new
/// column there should get a line here too.
export function fileSearchText(f: FileAnalysis): string {
  const parts = [
    f.file_name,
    f.format,
    f.ext_mismatch ? "extension mismatch" : "",
    f.declared_bits != null ? `${f.declared_bits}-bit` : "float",
    f.real_bit_depth != null ? `${f.real_bit_depth}-bit real` : "",
    `${(f.sample_rate / 1000).toFixed(1)}k`,
    fmtDuration(f.duration_secs),
    fmtSize(f.size_bytes),
    fmtCutoff(f),
    `${f.channels}ch`,
    f.fake_stereo == null
      ? f.channels <= 1
        ? "mono"
        : ""
      : f.fake_stereo
        ? "dual-mono fake stereo"
        : f.channels > 2
          ? "multi"
          : "stereo",
    f.clipping.clipped ? `${f.clipping.clip_events} clip events clipping` : "no clipping",
    Number.isFinite(f.clipping.true_peak_dbtp)
      ? `${f.clipping.true_peak_dbtp.toFixed(1)} dbtp true peak`
      : "",
    f.dr_db != null && Number.isFinite(f.dr_db) ? `${f.dr_db.toFixed(1)} db dynamics` : "",
    detectionSearchWords(f.detections),
    f.detections.detail,
    f.badge ?? "",
    md5SearchWords(f.flac_md5),
    f.error ?? "",
  ];
  return parts.filter(Boolean).join(" | ").toLowerCase();
}

/// True when every "word" in `query` — a run of letters/digits, with
/// anything else (spaces, hyphens, punctuation) treated as a separator — is
/// found as a whole word in `haystack`, except the last one, which only
/// needs to *start* a word if the query doesn't itself end on a separator
/// (i.e. it's still being typed).
///
/// The whole-word rule (rather than a plain substring search) is what stops
/// "16-" — typed while aiming for "16-bit" — from matching "160 MB" or
/// "116 MB": both contain "16" as a substring, but the trailing "-" closes
/// the word, and `\b16\b` doesn't match a "16" immediately followed by
/// another digit ("160") or preceded by one ("116"). Typing "16" without the
/// trailing separator still matches both, since at that point it could still
/// become "160" — the word isn't closed yet.
export function matchesSearch(haystack: string, query: string): boolean {
  const words = query.toLowerCase().match(/[a-z0-9]+/g);
  if (!words) return true;
  const lastWordOpen = /[a-z0-9]$/.test(query);
  const hay = haystack.toLowerCase();
  return words.every((word, i) => {
    const closed = i < words.length - 1 || !lastWordOpen;
    return new RegExp(`\\b${word}${closed ? "\\b" : ""}`).test(hay);
  });
}
